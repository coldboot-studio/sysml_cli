from __future__ import annotations

RELEASE_REPOSITORY = "https://github.com/Systems-Modeling/SysML-v2-Release"
RELEASE_BRANCH = "master"

GRAMMAR_REFERENCES = {
    "sysml_textual": {
        "path": "bnf/SysML-textual-bnf.kebnf",
        "url": "https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/SysML-textual-bnf.kebnf",
    },
    "kerml_textual": {
        "path": "bnf/KerML-textual-bnf.kebnf",
        "url": "https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/master/bnf/KerML-textual-bnf.kebnf",
    },
}

MODEL_PROJECTS = ("sysml", "kerml", "sysml.library")
SUPPORTED_EXTENSIONS = frozenset({".sysml", ".kerml"})
