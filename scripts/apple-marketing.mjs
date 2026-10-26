import { createPrivateKey, sign } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const directory = process.env.STORE_OUTPUT_DIR;
if (!directory) throw new Error('Set STORE_OUTPUT_DIR to a private backup directory.');
const required = ['ASC_KEY_ID', 'ASC_ISSUER_ID', 'ASC_KEY_P8'];
for (const name of required) {
  if (!process.env[name]) throw new Error(`Missing ${name}.`);
}
const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
const now = Math.floor(Date.now() / 1000);
const unsigned = `${encode({ alg: 'ES256', kid: process.env.ASC_KEY_ID, typ: 'JWT' })}.${encode({
  iss: process.env.ASC_ISSUER_ID,
  iat: now - 30,
  exp: now + 900,
  aud: 'appstoreconnect-v1',
})}`;
const key = createPrivateKey(Buffer.from(process.env.ASC_KEY_P8, 'base64').toString());
const signature = sign('sha256', Buffer.from(unsigned), { key, dsaEncoding: 'ieee-p1363' });
const token = `${unsigned}.${signature.toString('base64url')}`;
const origin = 'https://api.appstoreconnect.apple.com';
async function get(path) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Unexpected API origin.');
  const response = await fetch(url, { headers: { Authorization: `Bearer ${token}` } });
  const data = await response.json();
  if (!response.ok) throw new Error(`Apple API ${response.status}: ${JSON.stringify(data.errors)}`);
  return data;
}
async function save(name, data) {
  await writeFile(join(directory, name), `${JSON.stringify(data, null, 2)}\n`);
}
await mkdir(directory, { recursive: true, mode: 0o700 });
const apps = await get('/v1/apps?filter[bundleId]=app.poziomki.ios');
if (apps.data.length !== 1) throw new Error('Expected exactly one poziomki app.');
const app = apps.data[0];
await save('apple-app.json', app);
const versions = await get(`/v1/apps/${app.id}/appStoreVersions?filter[platform]=IOS&limit=20`);
await save('apple-versions.json', versions);
console.log('Apple versions:', versions.data.map(({ id, attributes: a }) => ({
  id, version: a.versionString, state: a.appStoreState,
})));
for (const version of versions.data.slice(0, 4)) {
  const localizations = await get(`/v1/appStoreVersions/${version.id}/appStoreVersionLocalizations?limit=50`);
  await save(`apple-${version.id}-localizations.json`, localizations);
  for (const localization of localizations.data) {
    const sets = await get(`/v1/appStoreVersionLocalizations/${localization.id}/appScreenshotSets?include=appScreenshots&limit=50`);
    await save(`apple-${version.id}-${localization.attributes.locale}-screenshots.json`, sets);
  }
}
