from playwright.sync_api import sync_playwright
from pathlib import Path
import json,time
root=Path(__file__).parent
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless=True,args=['--allow-file-access-from-files'])
 page=browser.new_page(viewport={'width':1280,'height':800})
 errors=[];requests=[]
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.on('request',lambda r:requests.append(r.url))
 page.goto((root/'chamber02.html').as_uri());page.wait_for_function('window.ready === true',timeout=60000);page.wait_for_timeout(2000)
 before=page.evaluate('({pos:demo.position,frames:demo.frames,ground:demo.grounded,errors:errors})')
 page.screenshot(path=str(root/'chamber00-browser.png'))
 page.keyboard.down('KeyW');page.wait_for_timeout(1000);page.keyboard.up('KeyW');after=page.evaluate('demo.position')
 assert after!=before['pos'],'walking did not change position'
 page.keyboard.down('Space');page.wait_for_timeout(150);jump=page.evaluate('demo.position');page.keyboard.up('Space');page.wait_for_timeout(1200)
 page.keyboard.press('KeyR');page.wait_for_timeout(200)
 page.locator('#start').click();page.wait_for_timeout(300);locked=page.evaluate('document.pointerLockElement?.id');yaw=page.evaluate('demo.yaw');page.mouse.move(600,300);page.mouse.move(740,360,steps=10);page.wait_for_timeout(200);look=page.evaluate('demo.yaw')
 page.keyboard.press('Escape');page.keyboard.press('KeyN');page.wait_for_timeout(100);noclip=page.evaluate('demo.noclip')
 result={'url':page.url,'before':before,'walk':after,'jump':jump,'pointer_lock':locked,'yaw_before':yaw,'yaw_after':look,'noclip':noclip,'page_errors':errors,'requests':requests,'state':page.evaluate('({frames:demo.frames,vertices:demo.vertices,brushes:demo.brushes,errors:errors})')}
 (root/'browser-verification-02.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2));assert not errors;assert locked=='c';assert yaw!=look;assert noclip;assert len(requests)==1
 browser.close()
