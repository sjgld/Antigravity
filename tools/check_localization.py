"""Check that Russian UI strings and draft translation markers cannot ship."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOKENS = re.compile(
    r'(?P<comment>//[^\n]*|/\*.*?\*/)|'
    r'(?P<raw>(?:br|r)(?P<hash>\#*)".*?"(?P=hash))|'
    r'(?P<string>b?"(?:\\.|[^"\\])*")', re.S)

# These are parser fixtures, localized OS error probes, or legacy log redaction
# patterns. They must retain their original bytes, and are never UI labels.
ALLOWED = {
    'src/launch_guard.rs': {'тест'},
    'src/ls_log.rs': {'Э'},
    'src/watchdog.rs': {'занят', 'отказано в доступе'},
    'src/gui/report.rs': {'свой прокси', 'выходит через', 'сменил выход на'},
    'src/resolvers.rs': {'свой DNS'},
}
failures = []
for path in sorted((ROOT / 'src').rglob('*.rs')):
    relative = path.relative_to(ROOT).as_posix()
    source = path.read_text(encoding='utf-8')
    for match in TOKENS.finditer(source):
        if match.group('comment'):
            continue
        literal = match.group()
        remaining = literal
        for allowed in ALLOWED.get(relative, ()):
            remaining = remaining.replace(allowed, '')
        if re.search('[А-Яа-яЁё]', remaining) or re.search(r'ZXQ\d+QXZ|@\d{4}@', literal):
            line = source.count('\n', 0, match.start()) + 1
            failures.append(f'{relative}:{line}: untranslated string')
if failures:
    raise SystemExit('\n'.join(failures))
print('Localization check passed: no untranslated Russian UI strings.')
