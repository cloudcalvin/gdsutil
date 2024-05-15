use std::cell::RefCell;

use gds21::{
    GdsArrayRef, GdsBoundary, GdsBox, GdsElement, GdsLibrary, GdsNode, GdsPath, GdsPoint,
    GdsStructRef, GdsTextElem,
};
use regex::RegexSet;

fn snap_xys(xys: &mut Vec<GdsPoint>, nm: i32) {
    for xy in xys {
        snap_xy(xy, nm)
    }
}

fn snap_xys_array(xys: &mut [GdsPoint], nm: i32) {
    for xy in xys {
        snap_xy(xy, nm)
    }
}


fn snap_xy(xy: &mut GdsPoint, nm: i32) {
    if nm < 0 {
        panic!("nm must be an integer tolerance value greater than zero")
    }
    println!("snap_to_grid_single x:  {} -> {}", xy.x, snap_to_grid_single(xy.x, nm));
    println!("snap_to_grid_single y:  {} -> {}", xy.y, snap_to_grid_single(xy.y, nm));
    xy.x = snap_to_grid_single(xy.x, nm);
    xy.y = snap_to_grid_single(xy.y, nm);
}

fn snap_to_grid_single(value: i32, nm: i32) -> i32 {
    // Divide the value by the grid size, round to the nearest whole number,
    // then multiply back by the grid size
    ((value as f32/ nm as f32).round() * nm as f32) as i32
    
}


fn snap_to_grid_recursive(
    component: &str,
    nm: i32,
    levels: i32,
    lib: &RefCell<&mut GdsLibrary>,
    re: &RegexSet,
    depth: i32
){
    
    for _struct in lib.borrow_mut().structs.as_mut_slice() {
        if _struct.name.starts_with(component) {
            for element in &mut _struct.elems {
                match element {
                    GdsElement::GdsBoundary(GdsBoundary { xy, .. }) => snap_xys(xy, nm),
                    GdsElement::GdsPath(GdsPath { xy, .. }) => snap_xys(xy, nm),
                    GdsElement::GdsStructRef(GdsStructRef { name , xy, .. }) => {
                        let re_match = re.matches(&name).matched_any();
                        if re_match  {
                            snap_xy(xy, nm);
                            if depth < levels {
                                snap_to_grid_recursive(name, nm, levels, &lib, re, depth + 1)
                            }
                        }
                    },
                    GdsElement::GdsArrayRef(GdsArrayRef { xy, .. }) => snap_xys_array(xy, nm),
                    GdsElement::GdsTextElem(GdsTextElem { xy, .. }) => snap_xy(xy, nm),
                    GdsElement::GdsNode(GdsNode { xy, .. }) => snap_xys(xy, nm),
                    GdsElement::GdsBox(GdsBox { xy, .. }) => snap_xys_array(xy, nm),
                }
            }
        }
    }
    
}

pub fn snap_to_grid(
    name: &str,
    nm: i32,
    levels: i32,
    lib: &mut GdsLibrary,
    re: &RegexSet,
    depth: i32
) {
    let ref_cell: RefCell<&mut GdsLibrary> = RefCell::new(lib);
    snap_to_grid_recursive(name, nm, levels, &ref_cell, re, depth)
    // let json = serde_json::to_string(&lib);
    // println!("{serde_json::to_string(json):?}");
    // println!("{}", json.unwrap());
}
