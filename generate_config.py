import sqlite3
import json
import os

db_path = r"C:\Users\PerVenT\AppData\Roaming\amaran Desktop\80414390_secure_id\amaran.db"
conn = sqlite3.connect(db_path)
cur = conn.cursor()

# Get mesh
mesh_row = cur.execute("SELECT uuid, net_key, app_key FROM mesh LIMIT 1;").fetchone()
mesh_uuid, net_key, app_key = mesh_row

# Get fixtures
fixtures = []
for row in cur.execute("SELECT name, mac_address, node_address, device_key, device_uuid, code FROM fixtures;").fetchall():
    name, mac, node_addr, dev_key, dev_uuid, code = row
    fixtures.append({
        "name": name,
        "mac": mac,
        "address": node_addr,
        "deviceKey": dev_key,
        "deviceUuid": dev_uuid,
        "code": code
    })

config = {
    "meshUuid": mesh_uuid,
    "netKey": net_key,
    "appKey": app_key,
    "lights": fixtures
}

out_path = r"C:\Users\PerVenT\.gemini\antigravity\scratch\amaran-control\config.json"
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(config, f, indent=2, ensure_ascii=False)

print("Saved config.json:")
print(json.dumps(config, indent=2))
