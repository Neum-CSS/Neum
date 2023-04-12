use crate::{html_parse, neum_parse, ARGS};
use itertools::Itertools;
use std::fs::File;
use std::io::Write;

pub fn update() {
    let mut output = String::from("/* auto generated by Neum https://github.com/AMTitan/Neum */\n");
    let html = html_parse::HTML_FILES.lock().unwrap();
    let neum = neum_parse::NEUM_FILES.lock().unwrap();

    let classes = html.values().clone();
    let neum = neum.values().clone();

    let mut total_classes = Vec::new();
    for i in classes {
        total_classes.append(&mut i.clone());
    }
    total_classes = total_classes
        .iter()
        .unique()
        .cloned()
        .collect::<Vec<String>>();

    let mut total_neum = neum::Neum::default();

    for i in neum {
        total_neum = total_neum.combine_priority(i.clone());
    }

    for i in total_classes {
        if let Some(x) = total_neum.convert(i.clone()) {
            output.push_str(&format!(".{i}{{{x}}}"));
        }
    }
    let mut file = File::create(ARGS.output.clone()).unwrap();
    file.write_all(output.as_bytes()).unwrap();
}
