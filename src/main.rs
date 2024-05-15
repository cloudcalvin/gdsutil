#![allow(dead_code, unused_imports)]

mod commands;

use commands::def_to_gds::convert_def_to_gds;
use commands::positions_to_file::extract_layout_data;
use commands::replace_all::replace_all;
use commands::snap_to_grid::snap_to_grid;

use clap::ArgAction;
use gds21::{GdsLibrary, GdsStruct};
use regex::RegexSet;
use std::default;

// #[macro_use]

fn main() {
    let cmd = clap::Command::new("gds")
        .bin_name("gdsu")
        .subcommand_required(true)
        .subcommand(
            clap::command!("print").arg(
                clap::arg!(<VALUE>)
                    .id("input")
                    .value_parser(clap::value_parser!(String)),
            ),
        )
        .subcommand(
            clap::command!("snap")
                .arg(
                    clap::arg!(<VALUE>)
                        .id("top")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    clap::arg!(--input <PATH>)
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(
                    clap::arg!(--"output" <PATH>)
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(clap::arg!(--"gridsize" <INT>).value_parser(clap::value_parser!(i32)))
                .arg(
                    clap::arg!(--"levels" <INT>)
                        .value_parser(clap::value_parser!(i32))
                        .default_value("1"),
                )
                .arg(
                    clap::arg!(-P --"patterns" <STRING>)
                        .num_args(0..)
                        .action(ArgAction::Append)
                        .value_parser(clap::value_parser!(String))
                        .required(false), // .min_values(1)
                ),
        )
        .subcommand(
            clap::command!("extract").subcommand(
                clap::command!("srefs")
                    .arg(
                        clap::arg!(<VALUE>)
                            .id("top")
                            .value_parser(clap::value_parser!(String)),
                    )
                    .arg(
                        clap::arg!(--input <PATH>)
                            .value_parser(clap::value_parser!(std::path::PathBuf)),
                    )
                    .arg(
                        clap::arg!(--"output" <PATH>)
                            .value_parser(clap::value_parser!(std::path::PathBuf)),
                    )
                    .arg(
                        clap::arg!(--"levels" <INT>)
                            .value_parser(clap::value_parser!(i32))
                            .default_value("1"),
                    )
                    .arg(
                        clap::arg!(-P --"patterns" <STRING>)
                            .action(ArgAction::Append)
                            .num_args(0..)
                            // .min_values(1)
                            .value_parser(clap::value_parser!(String))
                            .required(false),
                    ),
            ),
        )
        .subcommand(
            clap::command!("replace").subcommand(
                clap::command!("srefs")
                    .arg(
                        clap::arg!(<VALUE>)
                            .id("cell")
                            .value_parser(clap::value_parser!(String)),
                    )
                    .arg(
                        clap::arg!(--input <PATH>)
                            .value_parser(clap::value_parser!(std::path::PathBuf)),
                    )
                    .arg(
                        clap::arg!(--"output" <PATH>)
                            .value_parser(clap::value_parser!(std::path::PathBuf)),
                    )
                    .arg(
                        clap::arg!(--"replacements" <PATH>)
                            .value_parser(clap::value_parser!(std::path::PathBuf)),
                    )
                    .arg(
                        clap::arg!(--"levels" <INT>)
                            .value_parser(clap::value_parser!(i32))
                            .default_value("1"),
                    )
                    .arg(
                        clap::arg!(-P --"patterns" <STRING>)
                            .action(ArgAction::Append)
                            .num_args(0..)
                            .value_parser(clap::value_parser!(String))
                            .required(false),
                    ),
            ),
        )
        .subcommand(
            clap::command!("def2gds")
                .arg(
                    clap::arg!(<VALUE>)
                        .id("top")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    clap::arg!(--input <PATH>)
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(
                    clap::arg!(--"output" <PATH>)
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(
                    clap::arg!(--"lef" <PATH>)
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                ),
        );
    let matches = cmd.get_matches();
    match matches.subcommand() {
        Some(("print", matches)) => {
            let input: &String = matches.get_one::<String>("input").unwrap();
            let lib = GdsLibrary::load(input.to_owned()).unwrap();
            let json = serde_json::to_string(&lib);
            println!("{}", json.unwrap());
        }
        Some(("snap", matches)) => {
            let top: &String = matches.get_one::<String>("top").unwrap();
            let input = matches.get_one::<std::path::PathBuf>("input").unwrap();
            let mut lib = GdsLibrary::load(input.to_owned()).unwrap();
            let output = matches.get_one::<std::path::PathBuf>("output").unwrap();
            let gridsize = matches.get_one::<i32>("gridsize").unwrap();
            let levels = matches.get_one::<i32>("levels").unwrap();
            let patterns: Option<Vec<&str>> = matches
                .get_many::<String>("patterns")
                .map(|values_ref| values_ref.map(|s| s.as_str()).collect());
            let re = RegexSet::new(patterns.unwrap_or(vec![".*"]).into_iter()).unwrap();

            snap_to_grid(
                top,
                gridsize.to_owned(),
                levels.to_owned(),
                &mut lib,
                &re,
                1,
            );
            let result = lib.save(output.to_owned());
        }
        Some(("extract", matches)) => match matches.subcommand() {
            Some(("srefs", matches)) => {
                let top: &String = matches.get_one::<String>("top").unwrap();
                let input = matches.get_one::<std::path::PathBuf>("input").unwrap();
                let output = matches.get_one::<std::path::PathBuf>("output").unwrap();
                let levels = matches.get_one::<i32>("levels").unwrap();
                let patterns: Option<Vec<&str>> = matches
                    .get_many::<String>("patterns")
                    .map(|values_ref| values_ref.map(|s| s.as_str()).collect());
                println!(
                    "Extracting SREFs for top: {}, with patterns {:?}",
                    top,
                    patterns.as_ref().unwrap_or(&vec!(".*"))
                );
                extract_layout_data(top, input, output, levels, patterns).unwrap();
            }
            _ => unreachable!("clap should ensure we don't get here"),
        },
        Some(("replace", matches)) => match matches.subcommand() {
            Some(("srefs", matches)) => {
                let cell: &String = matches.get_one::<String>("cell").unwrap();
                let input = matches.get_one::<std::path::PathBuf>("input").unwrap();
                let output = matches.get_one::<std::path::PathBuf>("output").unwrap();
                let mut lib = GdsLibrary::load(input.to_owned()).unwrap();
                let replacements_csv = matches
                    .get_one::<std::path::PathBuf>("replacements")
                    .unwrap();
                let levels = matches.get_one::<i32>("levels").unwrap();
                let patterns: Option<Vec<&str>> = matches
                    .get_many::<String>("patterns")
                    .map(|values_ref| values_ref.map(|s| s.as_str()).collect());
                replace_all(
                    cell,
                    &mut lib,
                    Some(replacements_csv),
                    levels,
                    patterns,
                    false,
                )
                    .unwrap();
                let _ = lib.save(output.to_owned());
            }
            _ => unreachable!("clap should ensure we don't get here"),
        },
        Some(("def2gds", matches)) => {
            let top: &String = matches.get_one::<String>("top").unwrap();
            let input = matches.get_one::<std::path::PathBuf>("input").unwrap();
            // let mut lib = GdsLibrary::load(input.to_owned()).unwrap();
            let output = matches.get_one::<std::path::PathBuf>("output").unwrap();
            let lef = matches.get_one::<std::path::PathBuf>("lef").unwrap();
            let _ = convert_def_to_gds(&top, &input, &output, &[&lef]);
            // let result = lib.save(output.to_owned());
        }
        _ => unreachable!("clap should ensure we don't get here"),
    };
}
