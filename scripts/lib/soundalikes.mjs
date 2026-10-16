// CC-SENSE-CUE v1.1 F7 -- the ONE read API for sound-alike tables (node side).
//
// Same contract as tools/soundalikes.py and src/soundalikes.rs:
//   soundalikes(lang, voice?) -> { status, voice, generator, sets: [...] }
// A language with no table reads as "missing" with no sets. A voice that is not
// the one the table was built for also reads as "missing" (I6), but the sets
// stay, because H1 leaves Normal grading exactly as it is today.
// Nothing else opens assets/words/<lang>/soundalikes.json; soundalike-api-check
// fails the gate on any other reader.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

export function soundalikes(lang, voice = null, root = '.') {
  const p = join(root, 'assets', 'words', lang, 'soundalikes.json');
  if (!existsSync(p)) return { lang, status: 'missing', voice: null, generator: null, sets: [] };
  const t = JSON.parse(readFileSync(p, 'utf8'));
  if (voice !== null && t.voice !== voice) return { ...t, status: 'missing', voiceMismatch: true };
  return t;
}

/// Member-word lists of every set whose kind is in `kinds`. Grading, the bonus
/// and audio exemptions read ['true']; CC-IMPOSSIBLE reads ['true', 'accent'].
export function setsOfKind(lang, kinds = ['true'], voice = null, root = '.') {
  return soundalikes(lang, voice, root)
    .sets.filter((s) => kinds.includes(s.kind))
    .map((s) => s.members.map((m) => m.memberId));
}
