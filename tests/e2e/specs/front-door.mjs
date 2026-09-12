// front-door.spec — CC-ONBOARD-JR Phase B (F6, F7, F9, F11, F12).
//
// The invariants here are the ones a child's privacy rests on: I1 says no email
// or password field may render before an age verdict exists, and I2 says a
// locked junior can reach no account surface at all. Both are stated as "no
// such field is on the page", not "the screen looks right", because a field
// that renders offscreen is still a field that collected something.
//
// The account endpoints are stubbed: this suite proves the CLIENT's flow and
// gating. The server's own rules (code expiry, attempt limits, non-enumeration,
// the password blocklist) are proved in backend/test_auth_codes.py, against the
// real Flask app.
import { openApp, assert } from '../harness.mjs';

const ADULT = { y: 1990, m: 1, d: 1 };
const CHILD = { y: new Date().getFullYear() - 8, m: 1, d: 1 };

// Real visibility, and NOT via offsetParent: that is null for every
// `position: fixed` element, which is exactly what the front door is. The first
// draft used it and reported the open door as hidden -- and, far worse, made
// the three "no such field is on screen" assertions below pass for the wrong
// reason. A box with no hiding ancestor is what a player can actually see.
const IS_VISIBLE = `(el) => {
  if (!el) return false;
  const r = el.getBoundingClientRect();
  if (r.width === 0 || r.height === 0) return false;
  for (let n = el; n; n = n.parentElement) {
    const cs = getComputedStyle(n);
    if (cs.display === 'none' || cs.visibility === 'hidden' || cs.opacity === '0') return false;
  }
  return true;
}`;

const shown = (page, id) => page.evaluate(
  ([i, src]) => eval(src)(document.getElementById(i)), [id, IS_VISIBLE]);

// Every field a player could type an address or a password into that is
// actually on screen.
const visibleAuthFields = (page) => page.evaluate((src) => {
  const isVisible = eval(src);
  return [...document.querySelectorAll('input')]
    .filter((el) => ['email', 'password'].includes(el.type) && isVisible(el))
    .map((el) => el.id || el.name || el.type);
}, IS_VISIBLE);

async function answerBirthday(page, { y, m, d }) {
  await page.waitForSelector('#ageScrim.show', { timeout: 4000 });
  await page.selectOption('#ageYear', String(y));
  await page.selectOption('#ageMonth', String(m));
  await page.selectOption('#ageDay', String(d));
  await page.click('#ageSubmit');
  await page.waitForFunction(() => !document.getElementById('ageScrim').classList.contains('show'),
    null, { timeout: 4000 });
}

async function stubAuth(ctx, handlers = {}) {
  await ctx.route('**/api/auth/**', (route) => {
    const url = route.request().url();
    const key = Object.keys(handlers).find((k) => url.includes(k));
    const body = key ? handlers[key] : { ok: true };
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
  });
}

export async function run(browser, base, suite) {
  // Done 11 — a fresh install shows the birthday and nothing to type into.
  await suite.test('front_door_no_credential_field_before_a_verdict', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: '' });
    try {
      await page.waitForSelector('#ageScrim.show', { timeout: 4000 });
      assert(!(await shown(page, 'frontDoor')), 'the front door waits for the verdict');
      const visible = await visibleAuthFields(page);
      assert(visible.length === 0, `I1: no email/password field before a verdict, found ${JSON.stringify(visible)}`);
    } finally { await ctx.close(); }
  });

  // Done 12 (13+ half) — an adult birthday opens the front door, and the guest
  // link past it works, because play never requires an account (D1).
  await suite.test('front_door_opens_for_13_plus_and_can_be_walked_past', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: '' });
    try {
      await answerBirthday(page, ADULT);
      await page.waitForSelector('#frontDoor.show', { timeout: 5000 });
      assert(await shown(page, 'frontDoor'), 'a 13+ first launch meets the front door');
      for (const id of ['fdEmail', 'fdPassword', 'fdLogin', 'fdCreate', 'fdForgot', 'fdGuest']) {
        assert(await shown(page, id), `the mockup's ${id} is on the screen`);
      }
      const autocomplete = await page.evaluate(() => ({
        email: document.getElementById('fdEmail').getAttribute('autocomplete'),
        password: document.getElementById('fdPassword').getAttribute('autocomplete'),
        newPassword: document.getElementById('fdNewPassword').getAttribute('autocomplete'),
        rules: document.getElementById('fdNewPassword').getAttribute('passwordrules'),
      }));
      assert(autocomplete.email === 'email' && autocomplete.password === 'current-password',
        `iCloud Keychain needs real autocomplete values, got ${JSON.stringify(autocomplete)}`);
      assert(autocomplete.newPassword === 'new-password' && /minlength: 8/.test(autocomplete.rules || ''),
        'a suggested strong password must satisfy F9 (passwordrules)');

      await page.click('#fdGuest');
      await page.waitForFunction(() => !document.getElementById('frontDoor').classList.contains('show'),
        null, { timeout: 4000 });
      assert(!(await shown(page, 'frontDoor')), 'Play without an account leaves the door');
      assert(await shown(page, 'orbWrap'), 'and lands on the game');
    } finally { await ctx.close(); }
  });

  // Done 12 (under-13 half) / I1 / I2 — a child never meets an account surface.
  await suite.test('front_door_never_opens_for_under_13', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: '' });
    try {
      await answerBirthday(page, CHILD);
      await page.waitForFunction(() => document.body.classList.contains('kid'), null, { timeout: 4000 });
      assert(!(await shown(page, 'frontDoor')), 'under 13 goes straight to Spell Jr');
      assert(await page.evaluate(() => document.body.classList.contains('kid')), 'in Spell Jr');
      const visible = await visibleAuthFields(page);
      assert(visible.length === 0, `I2: no email/password field for a child, found ${JSON.stringify(visible)}`);
      assert(!(await shown(page, 'accountBtn')), 'no account entry point');
      assert(!(await shown(page, 'climbBtn')), 'no leaderboard entry point');
    } finally { await ctx.close(); }
  });

  // Done 21 / F12 — an existing install is a player mid-game, not a new signup.
  await suite.test('front_door_never_meets_an_existing_install', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      // An existing install must never open the door; give it real time to fail.
      await page.waitForTimeout(1200);
      assert(!(await shown(page, 'frontDoor')), 'an update must not drop a guest on a login screen');
      assert(!(await shown(page, 'ageScrim')), 'nor ask the birthday again');
    } finally { await ctx.close(); }
  });

  // F9 — the live checklist, on the field that sets a password.
  await suite.test('front_door_password_checklist_is_live', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', age: '' });
    try {
      await stubAuth(ctx);
      await answerBirthday(page, ADULT);
      await page.fill('#fdEmail', 'grown@example.com');
      await page.click('#fdCreate');
      await page.waitForFunction(() => document.getElementById('frontDoor').dataset.step === 'code',
        null, { timeout: 5000 });
      assert(await shown(page, 'fdCode'), 'the code step follows the address');
      await page.fill('#fdCode', '123456');
      await page.click('#fdCodeSubmit');
      await page.waitForFunction(() => document.getElementById('frontDoor').dataset.step === 'password',
        null, { timeout: 5000 });
      assert(await shown(page, 'fdNewPassword'), 'then the password step');

      const met = () => page.evaluate(() => ['fdRuleLen', 'fdRuleDigit', 'fdRuleSymbol']
        .filter((i) => document.getElementById(i).classList.contains('met')));
      await page.fill('#fdNewPassword', 'Example5');
      await page.waitForTimeout(120);
      assert(JSON.stringify(await met()) === JSON.stringify(['fdRuleLen', 'fdRuleDigit']),
        `length and digit met, symbol not: got ${JSON.stringify(await met())}`);
      await page.fill('#fdNewPassword', 'Example5%');
      await page.waitForTimeout(120);
      assert((await met()).length === 3, `all three rules met, got ${JSON.stringify(await met())}`);
    } finally { await ctx.close(); }
  });
}
