import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

if (!process.argv[2]) throw new Error('Usage: node marketing/build-review.mjs /absolute/path/review.html');
const directory = dirname(fileURLToPath(import.meta.url));
const root = resolve(directory, '..');
const output = resolve(process.argv[2]);
const escape = (value) => String(value).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const groups = [
  { id: 'google-phone', platform: 'google', title: 'Google Play, telefony', path: 'google/pl-PL/images/phoneScreenshots', labels: ['Poznawanie osób', 'Wydarzenia', 'Rozmowy', 'Wspólne zainteresowania', 'Mapa wydarzeń', 'Czat wydarzenia'] },
  { id: 'google-banner', platform: 'google', title: 'Google Play, baner', path: 'google/pl-PL/images/featureGraphic', labels: ['Grafika promocyjna'], wide: true },
  { id: 'google-seven', platform: 'google', title: 'Google Play, tablety 7 cali', path: 'google/pl-PL/images/sevenInchScreenshots', labels: ['Poznawanie osób', 'Wydarzenia'], tablet: true },
  { id: 'google-ten', platform: 'google', title: 'Google Play, tablety 10 cali', path: 'google/pl-PL/images/tenInchScreenshots', labels: ['Poznawanie osób', 'Wydarzenia'], tablet: true },
  { id: 'apple-phone', platform: 'apple', title: 'App Store, iPhone', path: 'apple/screenshots/pl-PL', prefix: 'iphone65-', labels: ['Poznawanie osób', 'Wspólne zainteresowania', 'Mapa wydarzeń', 'Poznajmy się na poziomkach'] },
  { id: 'apple-tablet', platform: 'apple', title: 'App Store, iPad', path: 'apple/screenshots/pl-PL', prefix: 'ipad129-', labels: ['Poznawanie osób', 'Wydarzenia'], tablet: true },
];
let count = 0;
const sections = [];
for (const group of groups) {
  const files = (await readdir(join(directory, group.path))).filter((name) => name.endsWith('.png') && (!group.prefix || name.startsWith(group.prefix))).sort();
  if (files.length !== group.labels.length) throw new Error('Unexpected image count: ' + group.id);
  const figures = [];
  for (const [index, file] of files.entries()) {
    const bytes = await readFile(join(directory, group.path, file));
    const width = bytes.readUInt32BE(16), height = bytes.readUInt32BE(20);
    const title = group.labels[index];
    const download = group.id + '-' + String(index + 1).padStart(2, '0') + '.png';
    figures.push(
      '<figure class="asset" data-name="' + escape(download) + '" data-title="' + escape(title) + '" data-size="' + width + ' × ' + height + '">',
      '<button class="open" type="button" aria-haspopup="dialog" aria-label="Powiększ: ' + escape(title) + '">',
      '<img loading="lazy" src="data:image/png;base64,' + bytes.toString('base64') + '" width="' + width + '" height="' + height + '" alt="' + escape(title) + '"></button>',
      '<figcaption><span>' + escape(title) + '</span><small>' + width + ' × ' + height + '</small><a class="download" href="#pobierz" download="' + escape(download) + '">Pobierz PNG</a></figcaption></figure>',
    );
    count++;
  }
  sections.push('<section class="image-group" data-platform="' + group.platform + '" id="' + group.id + '"><h2>' + group.title + '<span class="count">' + files.length + '</span></h2><div class="grid' + (group.wide ? ' wide' : group.tablet ? ' tablets' : '') + '">' + figures.join('') + '</div></section>');
}
const texts = [];
for (const [id, label, path] of [
  ['google-title', 'Google Play, nazwa', 'google/pl-PL/title.txt'],
  ['google-short', 'Google Play, krótki opis', 'google/pl-PL/short_description.txt'],
  ['google-full', 'Google Play, pełny opis', 'google/pl-PL/full_description.txt'],
  ['apple-promo', 'App Store, tekst promocyjny', 'apple/metadata/pl-PL/promotional_text.txt'],
  ['apple-description', 'App Store, opis', 'apple/metadata/pl-PL/description.txt'],
]) {
  const value = (await readFile(join(directory, path), 'utf8')).trim();
  texts.push('<div class="copy-block"><label for="' + id + '">' + label + '</label><textarea readonly id="' + id + '" rows="' + (value.length > 500 ? 10 : 3) + '">' + escape(value) + '</textarea><button class="action copy" type="button" data-copy="' + id + '">Skopiuj tekst</button></div>');
}
const regular = (await readFile(join(root, 'mobile/app-ui/src/commonMain/composeResources/font/nunito_regular.ttf'))).toString('base64');
const semibold = (await readFile(join(root, 'mobile/app-ui/src/commonMain/composeResources/font/nunito_semibold.ttf'))).toString('base64');
const css = [
  '@font-face{font-family:Nunito;src:url(data:font/ttf;base64,' + regular + ');font-weight:400;font-display:swap}',
  '@font-face{font-family:Nunito;src:url(data:font/ttf;base64,' + semibold + ');font-weight:600;font-display:swap}',
  '*{box-sizing:border-box}html{color-scheme:dark;scroll-behavior:smooth}body{margin:0;background:#090e10;color:#f0f6fc;font:17px/1.6 Nunito,sans-serif}main{max-width:1400px;margin:auto;padding:48px 28px 80px}',
  'h1{font-size:clamp(32px,5vw,56px);line-height:1.12;letter-spacing:-.03em;font-weight:600;margin:0 0 18px}h2{font-size:24px;line-height:1.3;font-weight:600;margin:0 0 24px}h3{font-size:20px;font-weight:600;margin:24px 0 8px}p{max-width:78ch;color:#b8c8ce;margin:12px 0}a{color:#67deed;text-underline-offset:4px}',
  'button,textarea{font:inherit}button,a{touch-action:manipulation}button{cursor:pointer}button:focus-visible,a:focus-visible,textarea:focus-visible{outline:2px solid #22d3ee;outline-offset:5px}.intro{margin-bottom:32px}.intro strong{font-weight:600;color:#f0f6fc}',
  '.filters{position:sticky;top:0;z-index:2;display:flex;flex-wrap:wrap;gap:24px;padding:18px 0;background:#090e10;border-bottom:1px solid #25383f}.filters button{background:none;border:0;border-bottom:2px solid transparent;color:#b8c8ce;padding:8px 0;font-weight:600}.filters button[aria-pressed=true]{color:#67deed;border-color:#22d3ee}.filters a{margin-left:auto;align-self:center}',
  '.image-group{padding-top:44px;scroll-margin-top:100px}.count{font-size:15px;font-weight:400;color:#9fb2ba;margin-left:12px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:24px;align-items:start}.grid.tablets{grid-template-columns:repeat(2,minmax(0,1fr));max-width:850px}.grid.wide{display:block;max-width:1024px}',
  'figure{margin:0;min-width:0}.open{display:block;width:100%;padding:0;border:0;background:transparent;text-align:left}.asset img{display:block;width:100%;height:auto;border-radius:8px}.open:hover img{outline:1px solid #67deed;outline-offset:3px}figcaption{display:flex;flex-direction:column;gap:3px;padding-top:14px;font-size:15px}figcaption small{font-size:13px;color:#9fb2ba}.download{align-self:flex-start;font-size:14px;margin-top:5px}',
  '[hidden]{display:none!important}.instructions{margin:44px 0 12px;border-bottom:1px solid #25383f;padding-bottom:24px}.instructions>summary{cursor:pointer;font-weight:600;font-size:20px}.instructions li{margin-bottom:10px;max-width:78ch;color:#b8c8ce}.instructions p{margin-top:12px}.instructions code{font-family:ui-monospace,monospace;font-size:.85em}pre{white-space:pre-wrap;overflow-wrap:anywhere;padding:18px;background:#111c21;border-radius:8px;max-width:980px}',
  '.metadata{border-top:1px solid #25383f;margin-top:56px;padding-top:32px;scroll-margin-top:100px}.copy-block{margin:28px 0;max-width:980px}.copy-block label{display:block;font-weight:600;margin-bottom:10px}textarea{display:block;width:100%;resize:vertical;border:1px solid #31484f;border-radius:8px;padding:16px;background:#101a1e;color:#e4edf0;font-size:15px;line-height:1.6}.action{background:#172a31;color:#f0f6fc;border:1px solid #39535d;border-radius:8px;padding:9px 15px;text-decoration:none;display:inline-block}.action:hover{background:#203d47}.copy{margin-top:10px}',
  'dialog{width:min(1120px,96vw);max-width:96vw;max-height:96vh;padding:18px;background:#101a1e;color:#f0f6fc;border:1px solid #39535d;border-radius:12px}dialog::backdrop{background:rgba(0,0,0,.9)}.dialog-head{display:flex;gap:16px;flex-wrap:wrap;align-items:center;margin-bottom:16px}.dialog-head strong{font-weight:600}.dialog-head .close{margin-left:auto}.dialog-image{display:block;width:100%;height:70vh;object-fit:contain}.dialog-nav{display:flex;gap:12px;align-items:center;justify-content:center;margin-top:16px}.dialog-nav span{color:#9fb2ba}.status{position:fixed;bottom:20px;left:50%;transform:translateX(-50%);background:#d8f8fc;color:#0b1d22;padding:12px 20px;border-radius:8px;z-index:5;max-width:90vw}',
  '@media(max-width:620px){main{padding:28px 18px 56px}.grid{grid-template-columns:repeat(2,minmax(0,1fr));gap:26px 16px}.grid.tablets{grid-template-columns:repeat(2,minmax(0,1fr))}.filters{gap:16px}.filters a{margin-left:0}.filters button{font-size:15px}h2{font-size:21px}.dialog-image{height:62vh}.dialog-head{gap:10px}.dialog-head strong{width:100%;font-size:15px}.dialog-head .close{margin-left:0}.dialog-nav{flex-wrap:wrap}.action{font-size:14px}}',
  '@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}}',
].join('\n');

function installGallery() {
  const assets = [...document.querySelectorAll('.asset')];
  const dialog = document.querySelector('dialog');
  const image = dialog.querySelector('img');
  let active = null;
  function visibleAssets() { return assets.filter((asset) => !asset.closest('section').hidden); }
  function show(asset) {
    active = asset;
    image.src = asset.querySelector('img').src;
    image.alt = asset.dataset.title;
    dialog.querySelector('strong').textContent = asset.dataset.title + ' · ' + asset.dataset.size;
    const download = dialog.querySelector('a');
    download.href = image.src;
    download.download = asset.dataset.name;
    const visible = visibleAssets();
    dialog.querySelector('.position').textContent = (visible.indexOf(asset) + 1) + ' / ' + visible.length;
    if (!dialog.open) dialog.showModal();
  }
  function move(offset) {
    const visible = visibleAssets();
    show(visible[(visible.indexOf(active) + offset + visible.length) % visible.length]);
  }
  for (const asset of assets) {
    asset.querySelector('.open').addEventListener('click', () => show(asset));
    asset.querySelector('.download').addEventListener('click', (event) => { event.currentTarget.href = asset.querySelector('img').src; });
  }
  for (const button of document.querySelectorAll('[data-filter]')) {
    button.addEventListener('click', () => {
      for (const item of document.querySelectorAll('[data-filter]')) item.setAttribute('aria-pressed', String(item === button));
      for (const section of document.querySelectorAll('.image-group')) section.hidden = button.dataset.filter !== 'all' && section.dataset.platform !== button.dataset.filter;
    });
  }
  dialog.querySelector('.close').addEventListener('click', () => dialog.close());
  dialog.querySelector('.previous').addEventListener('click', () => move(-1));
  dialog.querySelector('.next').addEventListener('click', () => move(1));
  dialog.addEventListener('click', (event) => { if (event.target === dialog) dialog.close(); });
  dialog.addEventListener('keydown', (event) => {
    if (event.key === 'ArrowLeft') { event.preventDefault(); move(-1); }
    if (event.key === 'ArrowRight') { event.preventDefault(); move(1); }
  });
  let timer;
  for (const button of document.querySelectorAll('[data-copy]')) {
    button.addEventListener('click', () => {
      const text = document.getElementById(button.dataset.copy);
      text.focus();
      text.select();
      let copied = false;
      try { copied = document.execCommand('copy'); } catch { copied = false; }
      const status = document.querySelector('.status');
      status.textContent = copied ? 'Tekst skopiowany.' : 'Tekst zaznaczony. Naciśnij Ctrl+C lub Cmd+C.';
      status.hidden = false; clearTimeout(timer);
      timer = setTimeout(() => { status.hidden = true; }, 4000);
    });
  }
}

const upload = [
  '<details class="instructions"><summary>Jak przesłać materiały do sklepów</summary>',
  '<h3>Google Play</h3><p>Play Console → aplikacja poziomki → Grow users → Store presence → Main store listing → język polski → Graphics. Dodaj baner, zrzuty telefonu oraz zrzuty tabletów w odpowiednich polach. Ustaw kolejność zgodną z numerami plików, zapisz i prześlij zmiany do sprawdzenia.</p>',
  '<p>Dotychczasowe przesyłanie przez API zatrzymywało się na zatwierdzeniu edycji, z błędem HTTP 403. W Play Console trzeba zweryfikować dostęp konta technicznego do aplikacji i uprawnienie Manage store presence. Sam kod błędu nie przesądza, którego uprawnienia brakuje.</p>',
  '<pre><code>gh workflow run android-release.yml --ref feat/eu-funding -f marketing=true</code></pre>',
  '<p>Workflow zapisuje kopię poprzednich materiałów i przesyła opis oraz grafiki. Nie buduje ani nie przesyła nowej wersji aplikacji.</p>',
  '<h3>App Store</h3><p>Najpierw właściciel konta musi zaakceptować zaległą umowę w <a href="https://appstoreconnect.apple.com/business">App Store Connect → Business</a>. Ostatnia próba API kończyła się błędem REQUIRED_AGREEMENTS_MISSING_OR_EXPIRED.</p>',
  '<p>Następnie: Apps → poziomki → nowa wersja iOS → język polski → App Previews and Screenshots. Dodaj 4 plansze iPhone i 2 iPad. Zmiana zrzutów zatwierdzonej aplikacji wymaga nowej wersji. Samo przesłanie materiałów do szkicu nie publikuje ich w sklepie.</p>',
  '<pre><code>gh workflow run ios-release.yml --ref feat/eu-funding &#92;\n  -f marketing=true -f marketing_upload=true -f app_store_version=0.22.7</code></pre>',
  '<p>Numer 0.22.7 jest przykładem. Użyj numeru następnego planowanego wydania. Workflow przesyła materiały do wersji roboczej, bez wysyłania IPA, zgłaszania do review i automatycznej publikacji.</p>',
  '<p>Dokumentacja: <a href="https://support.google.com/googleplay/android-developer/answer/9866151?hl=en">grafiki Google Play</a>, <a href="https://support.google.com/googleplay/android-developer/answer/9844686?hl=en">uprawnienia Play Console</a>, <a href="https://developer.apple.com/help/app-store-connect/manage-app-information/upload-app-previews-and-screenshots/">zrzuty App Store</a>, <a href="https://developer.apple.com/help/app-store-connect/manage-agreements/sign-and-update-agreements/">umowy Apple</a>.</p></details>',
].join('\n');
const html = [
  '<!doctype html><html lang="pl"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>poziomki, materiały marketingowe</title><style>' + css + '</style></head><body><main>',
  '<header class="intro"><h1>poziomki, materiały sklepowe</h1><p><strong>' + count + ' grafik</strong> oraz opisy do obu sklepów. Kliknij planszę, aby obejrzeć pełny rozmiar. Każdy PNG można pobrać osobno.</p><p>Android: ekrany lokalnego buildu z fikcyjnymi danymi. iPhone i iPad: rzeczywiste zrzuty listingu 0.22.6, w nowej oprawie. Ten plik działa offline.</p></header>',
  '<nav class="filters" aria-label="Filtruj grafiki"><button type="button" data-filter="all" aria-pressed="true">Wszystkie</button><button type="button" data-filter="google" aria-pressed="false">Google Play</button><button type="button" data-filter="apple" aria-pressed="false">App Store</button><a href="#teksty">Opisy do skopiowania</a></nav>',
  upload, sections.join('\n'),
  '<section class="metadata" id="teksty"><h2>Opisy do sklepów</h2><p>To teksty przygotowane do przesłania. Możesz skopiować każde pole osobno.</p>' + texts.join('\n') + '</section></main>',
  '<dialog aria-label="Podgląd grafiki"><div class="dialog-head"><strong></strong><a class="action" href="#pobierz" download>Pobierz PNG</a><button class="action close" type="button">Zamknij</button></div><img class="dialog-image" alt=""><div class="dialog-nav"><button class="action previous" type="button">Poprzednia</button><span class="position" aria-live="polite"></span><button class="action next" type="button">Następna</button></div></dialog><div class="status" role="status" hidden></div>',
  '<script>(' + installGallery.toString() + ')();</script></body></html>',
].join('\n');
await mkdir(dirname(output), { recursive: true });
await writeFile(output, html);
console.log('Saved ' + count + ' images and ' + texts.length + ' text fields to ' + output + ' (' + Buffer.byteLength(html) + ' bytes).');
