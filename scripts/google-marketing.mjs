import { createHash, createSign } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const mode = process.argv[2];
if (!['snapshot', 'validate', 'upload'].includes(mode)) {
  throw new Error('Usage: node scripts/google-marketing.mjs snapshot|validate|upload');
}
const root = resolve('marketing/google/pl-PL');
const kinds = ['featureGraphic', 'phoneScreenshots', 'sevenInchScreenshots', 'tenInchScreenshots'];
const listing = {};
const images = {};
if (mode !== 'snapshot') {
  for (const [field, file, limit] of [
    ['title', 'title.txt', 30],
    ['shortDescription', 'short_description.txt', 80],
    ['fullDescription', 'full_description.txt', 4000],
  ]) {
    const value = (await readFile(join(root, file), 'utf8')).trim();
    if (!value || [...value].length > limit) throw new Error(`Invalid ${file} length.`);
    listing[field] = value;
  }
  for (const kind of kinds) {
    const directory = join(root, 'images', kind);
    const files = (await readdir(directory)).filter((name) => name.endsWith('.png')).sort();
    if (!files.length || files.length > (kind === 'featureGraphic' ? 1 : 8)) {
      throw new Error(`Invalid image count: ${kind}.`);
    }
    images[kind] = [];
    for (const file of files) {
      const bytes = await readFile(join(directory, file));
      if (!bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) {
        throw new Error(`Not a PNG: ${file}.`);
      }
      const width = bytes.readUInt32BE(16);
      const height = bytes.readUInt32BE(20);
      if (kind === 'featureGraphic' ? width !== 1024 || height !== 500 :
        Math.min(width, height) < 320 || Math.max(width, height) > 3840) {
        throw new Error(`Invalid dimensions: ${kind}/${file} (${width}x${height}).`);
      }
      images[kind].push({ file, bytes, width, height, sha256: createHash('sha256').update(bytes).digest('hex') });
    }
  }
  console.log('Validated Polish listing and graphics:', Object.fromEntries(kinds.map((k) => [k, images[k].length])));
  if (mode === 'validate') process.exit(0);
}
const backup = process.env.STORE_OUTPUT_DIR;
if (!backup) throw new Error('Set STORE_OUTPUT_DIR to a private backup directory.');
await mkdir(backup, { recursive: true, mode: 0o700 });
const credentials = JSON.parse(process.env.PLAY_SERVICE_ACCOUNT_JSON || await readFile(process.env.GOOGLE_PLAY_SERVICE_ACCOUNT_FILE, 'utf8'));
const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
const now = Math.floor(Date.now() / 1000);
const unsigned = `${encode({ alg: 'RS256', typ: 'JWT' })}.${encode({
  iss: credentials.client_email,
  scope: 'https://www.googleapis.com/auth/androidpublisher',
  aud: 'https://oauth2.googleapis.com/token',
  iat: now - 30,
  exp: now + 1800,
})}`;
const signer = createSign('RSA-SHA256').update(unsigned);
const assertion = `${unsigned}.${signer.sign(credentials.private_key).toString('base64url')}`;
const tokenResponse = await fetch('https://oauth2.googleapis.com/token', {
  method: 'POST',
  body: new URLSearchParams({ grant_type: 'urn:ietf:params:oauth:grant-type:jwt-bearer', assertion }),
});
const token = await tokenResponse.json();
if (!tokenResponse.ok) throw new Error('Google authentication failed.');
const base = 'https://androidpublisher.googleapis.com/androidpublisher/v3/applications/app.poziomki';
const headers = { Authorization: `Bearer ${token.access_token}` };
async function request(url, method = 'GET', body, contentType = 'application/json') {
  const response = await fetch(url, {
    method,
    headers: { ...headers, 'Content-Type': contentType },
    body: body === undefined ? undefined : Buffer.isBuffer(body) ? body : JSON.stringify(body),
  });
  const data = response.status === 204 ? {} : await response.json();
  if (!response.ok) throw new Error(`Google API ${response.status}: ${data.error?.message}`);
  return data;
}
async function save(name, data) {
  await writeFile(join(backup, name), `${JSON.stringify(data, null, 2)}\n`);
}
const edit = await request(`${base}/edits`, 'POST', {});
let committed = false;
try {
  const before = await request(`${base}/edits/${edit.id}/listings`);
  await save('google-listings-before.json', before);
  if (!before.listings?.some((item) => item.language === 'pl-PL')) {
    throw new Error('Polish listing does not exist.');
  }
  for (const kind of kinds) {
    const beforeImages = await request(`${base}/edits/${edit.id}/listings/pl-PL/${kind}`);
    await save(`google-pl-PL-${kind}.json`, beforeImages);
    for (const [i, image] of (beforeImages.images || []).entries()) {
      const response = await fetch(`${image.url}=s0`);
      if (!response.ok) throw new Error(`Cannot back up ${kind}/${i}.`);
      await writeFile(join(backup, `google-pl-PL-${kind}-${i}.png`), Buffer.from(await response.arrayBuffer()));
    }
    if (mode === 'upload') {
      await request(`${base}/edits/${edit.id}/listings/pl-PL/${kind}`, 'DELETE');
      for (const image of images[kind]) {
        const upload = await request(
          `https://androidpublisher.googleapis.com/upload/androidpublisher/v3/applications/app.poziomki/edits/${edit.id}/listings/pl-PL/${kind}?uploadType=media`,
          'POST', image.bytes, 'image/png',
        );
        if (upload.image?.sha256 !== image.sha256) throw new Error(`Upload checksum mismatch: ${image.file}.`);
      }
    }
  }
  if (mode === 'upload') {
    await request(`${base}/edits/${edit.id}/listings/pl-PL`, 'PUT', { ...listing, language: 'pl-PL' });
    const result = await request(`${base}/edits/${edit.id}:commit`, 'POST');
    committed = true;
    await save('google-upload-result.json', { result, listing, uploadedAt: new Date().toISOString(),
      images: Object.fromEntries(kinds.map((k) => [k, images[k].map(({ file, sha256 }) => ({ file, sha256 }))])),
    });
    console.log('Google Play metadata edit committed; store review and publication timing apply.');
  }
} finally {
  if (!committed) await request(`${base}/edits/${edit.id}`, 'DELETE');
}
