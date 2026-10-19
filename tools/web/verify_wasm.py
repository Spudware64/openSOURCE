"""Exercise the real WASM game, not the old JavaScript scene demos."""
import argparse,json
from pathlib import Path
from playwright.sync_api import sync_playwright
parser=argparse.ArgumentParser()
parser.add_argument('--url',default='http://192.168.0.43:8080/wasm/')
parser.add_argument('--output',required=True)
parser.add_argument('--map',default='testchmb_a_00')
parser.add_argument('--headed',action='store_true')
parser.add_argument('--browser')
args=parser.parse_args()
out=Path(args.output);out.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
    browser=p.chromium.launch(executable_path=args.browser,headless=not args.headed,args=['--enable-webgl','--ignore-gpu-blocklist'])
    page=browser.new_page(viewport={'width':1280,'height':900})
    errors=[];logs=[]
    def on_error(error):
        errors.append(str(error))
        page.evaluate('window.smokeFailure=true')
    page.on('pageerror',on_error)
    page.on('console',lambda m:logs.append(m.type+': '+m.text))

    page.goto(args.url)
    page.select_option('#map',args.map)
    page.click('#start')
    try:
        page.wait_for_function('window.opensourceState?.frames > 10 || window.opensourceError || window.smokeFailure',timeout=120000)
        assert not page.evaluate('window.opensourceError'), page.evaluate('window.opensourceError')
        before=page.evaluate('window.opensourceState')

        page.bring_to_front()
        page.locator('canvas').click()
        page.wait_for_timeout(200)
        logs.append('pointer diagnostics: '+str(page.evaluate('({focus:document.hasFocus(),connected:document.querySelector("canvas").isConnected,lock:document.pointerLockElement?.tagName})')))
        page.keyboard.down('w');page.wait_for_timeout(1200);page.keyboard.up('w')
        after=page.evaluate('window.opensourceState')
        assert before['position']!=after['position'], 'Rust player did not move'
        page.keyboard.down('Space');page.wait_for_timeout(180);jump=page.evaluate('window.opensourceState');page.keyboard.up('Space')
        page.mouse.move(600,400);page.mouse.move(700,430,steps=10);page.wait_for_timeout(200)
        look=page.evaluate('window.opensourceState')
        input_mode=page.evaluate('document.pointerLockElement ? "pointer-lock" : window.opensourceBoundedMouse ? "bounded-fallback" : "none"')
        page.keyboard.press('Escape');page.wait_for_timeout(100)
        final=page.evaluate('window.opensourceState')
        page.screenshot(path=str(out/'wasm-final.png'))
        result={'input_mode':input_mode, 'url':page.url,'secure':page.evaluate('isSecureContext'),'before':before,'walk':after,'jump':jump,'look':look,'final':final,'page_errors':errors,'console':logs}
        print(json.dumps(result,indent=2));(out/'smoke.json').write_text(json.dumps(result,indent=2))
        assert not errors,errors
        assert before['yaw']!=look['yaw'],'mouse look did not update Rust yaw'
        assert not final['captured'],'escape did not release capture'
    except Exception:
        print(json.dumps({'errors':errors,'console':logs,'message':page.locator('#message').inner_text()},indent=2))
        page.screenshot(path=str(out/'wasm-failure.png'))
        raise
    finally:
        browser.close()
