use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::rename;
use csv;
use gds21::{GdsArrayRef, GdsBoundary, GdsBox, GdsElement, GdsLibrary, GdsPath, GdsPoint, GdsStructRef};
use regex::{Regex, RegexSet};
use serde::Serialize;
use serde_yaml;
use std::path::{Path, PathBuf};
use csv::{Reader, ReaderBuilder};
use toml;

fn read_replacements_csv<P: AsRef<Path>>(path: P) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut reader =  ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_path(path)?;
    let mut result = HashMap::new();

    for record in reader.records() {
        let record = record?;
        let key = record.get(0).unwrap_or_default().to_string();
        let value = record.get(1).unwrap_or_default().to_string();
        println!("Found: {}: {}", key, value);
        result.insert(key, value);
    }

    Ok(result)
}

pub fn replace_all(
    cell: &str,
    lib: &mut GdsLibrary,
    replacements_csv: Option<&PathBuf>,     // csv
    levels: &i32,
    patterns: Option<Vec<&str>>,
    in_place: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    
    // if a pattern doesnt suffice you can supply pairs of names for replacement
    let replacements = read_replacements_csv(replacements_csv.unwrap())?;

    println!("Replacing SREFs for cell: {}", cell);
    println!("Replacements {:?}", replacements.clone().into_iter());
    println!("Override patterns {:?}", patterns.as_ref().unwrap_or(&vec!(".*")));

    // let mut lib = GdsLibrary::load(input.to_owned()).unwrap();
    let ref_cell: RefCell<&mut GdsLibrary> = RefCell::new(lib);

    // let ref_cell: RefCell<&mut GdsLibrary> = RefCell::new(lib);

    // let mut results: Vec<Element> = vec![];
    let re = RegexSet::new(patterns.unwrap_or(vec![".*"]).into_iter()).unwrap();

    // replacement involves adding the new cells, removing the old cells,
    // and replacing all references to old elements with the name of the new elements

    for _struct in ref_cell.borrow_mut().structs.as_mut_slice() {
        if _struct.name.starts_with(cell) {
            for element in &mut _struct.elems {
                match element {
                    // GdsElement::GdsBoundary(GdsBoundary { xy, .. }) => snap_xys(xy, nm),
                    // GdsElement::GdsPath(GdsPath { xy, .. }) => snap_xys(xy, nm),
                    GdsElement::GdsStructRef(GdsStructRef { ref mut name , xy, .. }) => {
                        let re_match = re.matches(&name).matched_any();

                        if re_match  {
                            println!("attempting replacement of {}", &name);

                            if !replacements.contains_key(name) {
                                panic!("lookup failed")
                            }
                            let replacement = replacements.get(name).unwrap();
                            println!("replacement of {} with {} succeeded", &name, &replacement);

                            // println!("replacing reference to {}, with {}", &name, &replacement)
                            *name = replacement.to_owned();
                            // *element = GdsElement::GdsStructRef(GdsStructRef {
                            //     name: replacement.to_owned(),
                            //     // copy other fields
                            //     ..sref.clone()
                            // });
                        }
                    },
                    // GdsElement::GdsArrayRef(GdsArrayRef { xy, .. }) => snap_xys_array(xy, nm),
                    // GdsElement::GdsTextElem(GdsTextElem { xy, .. }) => snap_xy(xy, nm),
                    // GdsElement::GdsNode(GdsNode { xy, .. }) => snap_xys(xy, nm),
                    // GdsElement::GdsBox(GdsBox { xy, .. }) => snap_xys_array(xy, nm),
                    _ => {}
                }
            }
        }
        if replacements.contains_key(&_struct.name) {

        }
    }
    // for r in replacements.keys(){
    //     ref_cell.borrow_mut().structs.as_mut_slice().
    //
    // }

    Ok(())
}
