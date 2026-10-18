//! Installed-original-asset lifecycle audit; includes advanced and escape maps.
use opensource::{assets::Assets,entity::{Runtime,parse_entities}};
#[test]
fn all_installed_maps_preserve_inventory_and_remove_only_loadout_copies(){
 let fs=Assets::from_game("../portal-original/portal",None).unwrap();
 let mut count=0;
 for entry in std::fs::read_dir("../portal-original/portal/maps").unwrap(){
 let path=entry.unwrap().path();if path.extension().is_none_or(|e|e!="bsp"){continue;}
 let map=path.file_stem().unwrap().to_str().unwrap();let b=fs.read(&format!("maps/{map}.bsp")).unwrap();
 let entities=parse_entities(opensource::lump(&b,0,1).unwrap()).unwrap();
 let mut r=Runtime::new(&b).unwrap();r.direct_launch_loadout();
 let expected=if map.ends_with("_00") && map.starts_with("testchmb") || map=="testchmb_a_01" || map.starts_with("background"){[false,false]}else if matches!(map,"testchmb_a_02"|"testchmb_a_03"|"testchmb_a_04"|"testchmb_a_05"|"testchmb_a_06"|"testchmb_a_07"){[true,false]}else{[true,true]};
 assert_eq!(r.portal_inventory,expected,"direct {map}");
 let mut previous=Runtime::new(&b).unwrap();previous.portal_inventory=[true,true];
 let mut next=Runtime::new(&b).unwrap();next.enter_transition(&previous,glam::Vec3::ZERO);assert_eq!(next.portal_inventory,[true,true]);
 for e in entities.iter().filter(|e|e.get("classname")==Some("weapon_portalgun")){
 if let Some(name)=e.get("targetname"){if let Some(slot)=next.entity_model(name){
 let puzzle=e.get("parentname").is_some() || e.pairs.iter().any(|(k,_)|k=="OnPlayerPickup");
 assert_eq!(next.visible[slot],puzzle,"transition {map} {name}");}}
 }
 println!("{map}: direct {:?}; transition copies hidden, puzzles retained",r.portal_inventory);count+=1;
 }
 assert_eq!(count,26);
}
