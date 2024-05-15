use gds21::{GdsBoundary, GdsBox, GdsElement, GdsLibrary, GdsPath, GdsPoint, GdsStructRef};
use regex::{Regex, RegexSet};
use serde::Serialize;
use serde_yaml;
use std::path::PathBuf;
use toml;

enum LayoutDataType {
    POSITION,
    ROTATION,
    REFLECTION,
    MAGNIFICATION,
}

#[derive(Serialize)]
pub struct Element {
    name: String,
    layout: ElementLayout
}

#[derive(Serialize)]
pub struct ElementLayout {
    position: GdsPoint,
    rotation: f64,
    scale: f64,
    mirrored: bool,
}

pub fn extract_layout_data(
    top: &str,
    input: &PathBuf,
    output: &PathBuf,
    levels: &i32,
    patterns: Option<Vec<&str>>,
) -> Result<Vec<Element>, Box<dyn std::error::Error>> {
    let lib = GdsLibrary::load(input.to_owned()).unwrap();
    let mut results: Vec<Element> = vec![];
    let re = RegexSet::new(
        patterns.unwrap_or(vec![".*"]).into_iter()).unwrap();

    for s in lib.structs.as_slice() {
        if s.name.starts_with(top) {
            println!("found struct that starts with pattern => {}", top);
            for element in &s.elems {
                match element {
                    // GdsElement::GdsBoundary(GdsBoundary { xy, .. }) => {
                    //     // snap_xys(xy, *nm)
                    // },
                    // GdsElement::GdsPath(GdsPath { xy, .. }) => {
                    //     // snap_xys(xy, *nm)
                    // },
                    // // GdsElement::GdsArrayRef(GdsArrayRef { xy, .. }) => process_xy(xy.to_vec()),
                    // // GdsElement::GdsTextElem(GdsTextElem { xy, .. }) => snap_xy(vec!{xy}, &gridsize),
                    // // GdsElement::GdsNode(GdsNode { xy, .. }) => snap_xy(xy, &gridsize),
                    // GdsElement::GdsBox(GdsBox { xy, .. }) => {
                    //     // snap_xys_array(xy, *nm)
                    // },
                    GdsElement::GdsStructRef(GdsStructRef {
                        name, xy, strans, ..
                    }) => {
                        let re_match = re.matches(&name).matched_any();
                        if re_match { 
                            if strans.is_some() {
                                results.push(Element {
                                    name: name.to_owned(),
                                    layout: ElementLayout {
                                        position: xy.to_owned(),
                                        rotation: strans.as_ref().unwrap().angle.unwrap_or(0.0),
                                        scale: strans.as_ref().unwrap().mag.unwrap_or(1.0),
                                        mirrored: strans.as_ref().unwrap().reflected
                                    }
                                });
                            } else {
                                results.push(Element {
                                    name: name.to_owned(),
                                    layout: ElementLayout {
                                        position: xy.to_owned(),
                                        rotation: 0.0,
                                        scale: 1.0,
                                        mirrored: false
                                    }
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let json = serde_json::to_string(&results);
    // let result = lib.save(output.to_owned());
    if output.extension().unwrap() == "yaml" {
        save_as_yaml(&results, &output)?
    } else if output.extension().unwrap() == "toml" {
        save_as_toml(&results, &output)?
    } else {
        panic!()
    }
    Ok(results)
}

fn save_as_yaml<T: Serialize>(data: &Vec<T>, output: &PathBuf) -> Result<(), serde_yaml::Error> {
    let yaml_str = serde_yaml::to_string(&data).unwrap();
    std::fs::write(output, yaml_str).unwrap();
    Ok(())
}

fn save_as_toml<T: Serialize>(data: &Vec<T>, output: &PathBuf) -> Result<(), toml::ser::Error> {
    let toml_str = toml::to_string(&data).unwrap();
    std::fs::write(output, toml_str).unwrap();
    Ok(())
}
