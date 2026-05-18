use std::fmt::Write as _;

use crate::report::json_escape;

pub const RELEASE_REPOSITORY: &str = "https://github.com/Systems-Modeling/SysML-v2-Release";
pub const RELEASE_BRANCH: &str = "master";

pub struct PublicCorpus {
    pub name: &'static str,
    pub url: &'static str,
    pub license_note: &'static str,
    pub description: &'static str,
}

pub const PUBLIC_CORPORA: &[PublicCorpus] = &[
    PublicCorpus {
        name: "Systems-Modeling/SysML-v2-Release",
        url: "https://github.com/Systems-Modeling/SysML-v2-Release/tree/master/sysml/src",
        license_note: "EPL-2.0 for included models; see repository LICENSE.",
        description: "Official release examples, training models, validation models, and normative libraries.",
    },
    PublicCorpus {
        name: "GfSE/SysML-v2-Models",
        url: "https://github.com/GfSE/SysML-v2-Models",
        license_note: "BSD-3-Clause by default, with per-model overrides allowed.",
        description: "Community-curated collection of reusable SysML v2 textual models.",
    },
    PublicCorpus {
        name: "sensmetry/advent-of-sysml-v2",
        url: "https://github.com/sensmetry/advent-of-sysml-v2",
        license_note: "MIT; see repository LICENSE.",
        description: "Training-course examples organized by lesson, including model folders.",
    },
    PublicCorpus {
        name: "sensmetry/smart-home-hub-example",
        url: "https://github.com/sensmetry/smart-home-hub-example",
        license_note: "MIT; see repository LICENSE.",
        description: "Small complete SysML v2 project for smart-home hub architecture variants.",
    },
    PublicCorpus {
        name: "OMG SysML machine-readable files",
        url: "https://www.omg.org/spec/SysML/machine-readable",
        license_note: "OMG specification artifact licensing applies.",
        description: "Machine-readable SysML 2.0 artifacts, including the Simple Vehicle Model.",
    },
];

pub fn print_grammar_text() {
    println!("Repository: {RELEASE_REPOSITORY}");
    println!("Branch: {RELEASE_BRANCH}");
    println!("Model projects: sysml, kerml, sysml.library");
    println!("Grammar references:");
    println!("  sysml_textual: bnf/SysML-textual-bnf.kebnf");
    println!("    https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/SysML-textual-bnf.kebnf");
    println!("  kerml_textual: bnf/KerML-textual-bnf.kebnf");
    println!("    https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/KerML-textual-bnf.kebnf");
}

pub fn print_grammar_json() {
    println!(
        "{{\n  \"repository\": \"{}\",\n  \"branch\": \"{}\",\n  \"model_projects\": [\"sysml\", \"kerml\", \"sysml.library\"],\n  \"grammars\": {{\n    \"sysml_textual\": {{\n      \"path\": \"bnf/SysML-textual-bnf.kebnf\",\n      \"url\": \"https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/SysML-textual-bnf.kebnf\"\n    }},\n    \"kerml_textual\": {{\n      \"path\": \"bnf/KerML-textual-bnf.kebnf\",\n      \"url\": \"https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/KerML-textual-bnf.kebnf\"\n    }}\n  }}\n}}",
        json_escape(RELEASE_REPOSITORY),
        json_escape(RELEASE_BRANCH)
    );
}

pub fn print_corpus_text() {
    println!("Public SysML v2 model corpora:");
    for corpus in PUBLIC_CORPORA {
        println!("  {}", corpus.name);
        println!("    {}", corpus.url);
        println!("    {}", corpus.description);
        println!("    {}", corpus.license_note);
    }
}

pub fn print_corpus_json() {
    let mut output = String::from("{\n  \"corpora\": [\n");
    for (index, corpus) in PUBLIC_CORPORA.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        write!(
            output,
            "    {{\"name\": \"{}\", \"url\": \"{}\", \"description\": \"{}\", \"license_note\": \"{}\"}}",
            json_escape(corpus.name),
            json_escape(corpus.url),
            json_escape(corpus.description),
            json_escape(corpus.license_note)
        )
        .expect("write to String cannot fail");
    }
    output.push_str("\n  ]\n}");
    println!("{output}");
}
