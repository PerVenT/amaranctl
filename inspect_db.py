import sqlite3
import json
import sys

sys.stdout.reconfigure(encoding="utf-8")
db_path = r"C:\Users\PerVenT\AppData\Roaming\amaran Desktop\80414390_secure_id\amaran.db"
conn = sqlite3.connect(db_path)
cur = conn.cursor()

tables = [r[0] for r in cur.execute("SELECT name FROM sqlite_master WHERE type='table';").fetchall()]
print("Found tables:", tables)

for t in tables:
    count = cur.execute(f'SELECT COUNT(*) FROM "{t}";').fetchone()[0]
    cols = [c[1] for c in cur.execute(f'PRAGMA table_info("{t}");').fetchall()]
    print(f"\n--- Table: {t} ({count} rows) ---")
    print("Columns:", cols)
    if count > 0:
        rows = cur.execute(f'SELECT * FROM "{t}" LIMIT 3;').fetchall()
        for r in rows:
            print("Row:", r)
