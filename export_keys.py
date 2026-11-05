"""
Sidus Link Mesh Config Exporter
Extracts Bluetooth Mesh NetKey, AppKey, and Light MAC/Node Address
directly from Sidus Link Cloud API into config.json.
Uses only standard library (no pip install needed).
"""

import getpass
import json
import sys
import urllib.error
import urllib.request

REGIONS = ["eu", "us", "cn"]

HEADERS_BASE = {
    "Content-Type": "application/json",
    "platform": "2",
    "request_source": "4",
    "User-Agent": "okhttp/4.9.1",
}


def login_and_fetch(email, password, area="389"):
    token = None
    active_region = None

    # Try regions
    for reg in REGIONS:
        login_url = f"https://{reg}.sidus.link/uc/user/login"
        payload = json.dumps(
            {
                "Area": str(area).replace("+", ""),
                "Email": email.strip(),
                "Password": password,
                "PhoneNumber": "",
            }
        ).encode("utf-8")

        req = urllib.request.Request(login_url, data=payload, headers=HEADERS_BASE)
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8"))

                # Check response code (20000 is success in Sidus API)
                code = data.get("code")
                if code in (0, 200, 20000):
                    token = data.get("data", {}).get("token") or data.get("data", {}).get("Token")
                    active_region = reg
                    print(f"[+] Успішна авторизація через регіон: {reg.upper()} (код {code})")
                    break
                elif code == 4001:
                    # Redirect code
                    other = data.get("data", {}).get("OtherRegionUser", [])
                    print(
                        f"[*] Регіон {reg.upper()} вказав на перенаправлення (4001), шукаємо далі..."
                    )
                    continue
                else:
                    msg = data.get("msg", "Невідома помилка")
                    print(f"[-] Сервер {reg.upper()} повернув: {msg} (code {code})")
                    if "data" in data and data["data"]:
                        print(f"    Деталі: {data['data']}")
        except urllib.error.HTTPError as e:
            err_body = e.read().decode("utf-8", errors="replace")
            print(f"[-] HTTP {e.code} на {reg.upper()}: {err_body}")
        except Exception as e:
            print(f"[-] Помилка з'єднання з {reg.upper()}: {e}")

    if not token or not active_region:
        print("[!] Не вдалося увійти в акаунт на жодному сервері.")
        print("    Перевірте email, пароль та код країни.")
        return None

    # Save token for diagnostics
    try:
        with open("token.txt", "w", encoding="utf-8") as tf:
            tf.write(token)
    except Exception:
        pass

    # Endpoints to check
    endpoints_to_try = [
        "/amaran/pack/query",
        "/sidus/pack/query",
        "/project/pack/query",
        "/amaran/project/query",
        "/amaran/project/list",
        "/pro/pack/query",
        "/pro/project/query"
    ]

    for reg in ["eu", "us", "cn"]:
        print(f"[*] Перевіряємо сервер {reg.upper()}...")
        for ep in endpoints_to_try:
            url = f"https://{reg}.sidus.link{ep}"
            headers = {
                "Token": token,
                "token": token,
                "Authorization": f"Bearer {token}",
                "platform": "2",
                "request_source": "4",
                "User-Agent": "okhttp/4.9.1"
            }
            req = urllib.request.Request(url, headers=headers)
            try:
                with urllib.request.urlopen(req, timeout=5) as resp:
                    d = json.loads(resp.read().decode("utf-8"))
                    code = d.get("code")
                    if code in (0, 200, 20000):
                        payload = d.get("data")
                        sanitized_ep = ep.replace('/', '_')
                        with open(f"resp_{reg}_{sanitized_ep}.json", "w", encoding="utf-8") as rf:
                            json.dump(d, rf, indent=2, ensure_ascii=False)

                        if isinstance(payload, dict):
                            m = payload.get("meshs", {}).get("existing", []) or payload.get("mesh_list", [])
                            f = payload.get("fixtures", {}).get("existing", []) or payload.get("fixture_list", [])
                            p = payload.get("projects", {}).get("existing", []) or payload.get("project_list", [])
                            if m or f or p:
                                print(f"[+] ЗНАЙДЕНО ДАНІ на https://{reg}.sidus.link{ep}!")
                                print(f"    Мереж: {len(m)}, Ліхтарів: {len(f)}, Проєктів: {len(p)}")
                                return payload
                        elif isinstance(payload, list) and len(payload) > 0:
                            print(f"[+] ЗНАЙДЕНО СПИСОК на https://{reg}.sidus.link{ep}! Елементів: {len(payload)}")
                            return {"items": payload}
            except urllib.error.HTTPError:
                pass
            except Exception:
                pass

    print("[!] Жоден ендпоінт на EU/US/CN не повернув активних ліхтарів.")
    return None


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    print("=" * 60)
    print("  Експорт налаштувань Sidus Mesh для Amaran (Sidus Link API)")
    print("=" * 60)
    print("Введіть облікові дані вашого акаунта Sidus Link:")

    email = input("Email: ").strip()
    if not email:
        print("Email не може бути порожнім!")
        return

    password = getpass.getpass("Пароль: ").strip()
    if not password:
        print("Пароль не може бути порожнім!")
        return

    area = input(
        "Телефонний код країни (за замовчуванням 380 для України, Enter щоб залишити): "
    ).strip()
    if not area:
        area = "380"

    print("\n[*] З'єднуємося з серверами Sidus Link...")
    result = login_and_fetch(email, password, area)

    if not result:
        print("\n[!] Не вдалося отримати конфігурацію.")
        return

    # Debug raw response
    with open("raw_response.json", "w", encoding="utf-8") as fp:
        json.dump(result, fp, indent=2, ensure_ascii=False)

    meshs = result.get("meshs", {}).get("existing", [])
    fixtures = result.get("fixtures", {}).get("existing", [])

    if not meshs:
        print("\n[!] У вашому акаунті не знайдено створеної Mesh-мережі.")
        print(f"    Структура відповіді від сервера містить ключі: {list(result.keys())}")
        print("    Збережено сирі дані у raw_response.json для діагностики.")
        return

    primary_mesh = meshs[0]
    net_key = primary_mesh.get("net_key")
    app_key = primary_mesh.get("app_key")

    print("\n" + "=" * 60)
    print("[+] УСПІШНО ЗНАЙДЕНО КЛЮЧІ MESH МЕРЕЖІ:")
    print(f"    NetKey: {net_key}")
    print(f"    AppKey: {app_key}")
    print("=" * 60)

    lights_list = []
    print("\n[+] Знайдені світлові прилади:")
    for f in fixtures:
        name = f.get("name") or f.get("model") or "Amaran Light"
        mac = f.get("mac") or f.get("bluetooth_address") or ""
        node_addr = f.get("node_address") or f.get("unicast_address")
        print(f"  - Прилад: {name} | MAC: {mac} | Адреса в Mesh: {node_addr}")
        lights_list.append({"name": name, "mac": mac, "address": node_addr, "raw": f})

    config = {
        "netKey": net_key,
        "appKey": app_key,
        "lights": lights_list,
        "raw_mesh": primary_mesh,
    }

    output_filename = "config.json"
    with open(output_filename, "w", encoding="utf-8") as fp:
        json.dump(config, fp, indent=2, ensure_ascii=False)

    print(f"\n[+] Файл налаштувань успішно збережено в: {output_filename}")
    print("[*] Тепер ці ключі будуть використовуватись нашим додатком на Rust.")
    print("=" * 60)


if __name__ == "__main__":
    main()
