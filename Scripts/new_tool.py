#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Pascal Klose
# SPDX-License-Identifier: GPL-3.0-or-later

"""Scaffold a new tool page and wire it into the shared integration points.

Usage: new_tool.py <Name> <module> <slug> "<Menu text>" <LucideIcon> "<Menu group>" ["<search tags>"]
Example: new_tool.py Compression compression compression "Compress" FileArchive "Data & Formats" "compress gzip zlib"

Creates the page skeletons (ui/pages/<slug>.slint, src/pages/<module>.rs, src/units/<module>.rs) and
adds the enum entry, page index, global state, imports, menu entry (at the end of its group), page switch
and registration. The group must already exist in the menu; the optional
search tags are space-separated keywords the navigation search matches besides the name and group.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


# Every change is staged here and written at the end, so a refused run leaves the tree untouched.
STAGED: dict[str, str] = {}


def read(path: str) -> str:
    return STAGED.get(path) or (ROOT / path).read_text()


def write(path: str, text: str) -> None:
    STAGED[path] = text


def insert_after_last(text: str, pattern: str, addition: str, what: str) -> str:
    """Insert `addition` after the last line matching `pattern`."""
    matches = list(re.finditer(pattern, text, re.MULTILINE))
    if not matches:
        sys.exit(f"Anchor not found: {what}")
    end = matches[-1].end()
    return text[:end] + addition + text[end:]


def main() -> None:
    if len(sys.argv) not in (7, 8):
        sys.exit(__doc__)
    name, module, slug, menu, icon, group = sys.argv[1:7]
    tags = sys.argv[7] if len(sys.argv) == 8 else ""

    if not re.fullmatch(r"[A-Z][A-Za-z0-9]*", name):
        sys.exit("Name must be PascalCase")
    for path in (f"ui/pages/{slug}.slint", f"src/pages/{module}.rs", f"src/units/{module}.rs"):
        if (ROOT / path).exists():
            sys.exit(f"{path} already exists")
    if re.search(rf"\b{name}(State|Page)\b|AppPages\.{name}\b", read("ui/app-window.slint")):
        sys.exit(f"{name} is already wired in")

    # Enum entry.
    types = read("ui/types.slint")
    types, count = re.subn(r"(export enum AppPages \{[^}]*?) \}", rf"\1, {name} }}", types, count=1)
    if count != 1:
        sys.exit("AppPages enum not found")
    write("ui/types.slint", types)

    # Page index (persisted in the config) for both directions of the mapping.
    config = read("src/config.rs")
    last = max(int(n) for n in re.findall(r"=> (\d+),", config))
    index = last + 1
    config = insert_after_last(
        config, rf"^\s+AppPages::\w+ => {last},\n", f"            AppPages::{name} => {index},\n", "page to index"
    )
    config = insert_after_last(
        config, rf"^\s+{last} => AppPages::\w+,\n", f"            {index} => AppPages::{name},\n", "index to page"
    )
    write("src/config.rs", config)

    # Registration.
    main_rs = read("src/main.rs")
    main_rs = insert_after_last(
        main_rs,
        r"^\s+pages::(?!info|settings)\w+::register\(&app_window\);\n",
        f"    pages::{module}::register(&app_window);\n",
        "register calls",
    )
    write("src/main.rs", main_rs)
    mod_rs = read("src/pages/mod.rs")
    write("src/pages/mod.rs", mod_rs.rstrip("\n") + f"\npub mod {module};\n")
    units_rs = read("src/units/mod.rs")
    write("src/units/mod.rs", units_rs.rstrip("\n") + f"\npub mod {module};\n")

    # Window: state import, page import, state export, menu entry and page switch.
    window = read("ui/app-window.slint")
    window = window.replace('} from "globals.slint";', f'    {name}State,\n}} from "globals.slint";', 1)
    window = insert_after_last(
        window,
        r'^import \{ \w+Page \} from "pages/[\w-]+\.slint";\n',
        f'import {{ {name}Page }} from "pages/{slug}.slint";\n',
        "page imports",
    )
    window, count = re.subn(r"(export \{[^}]*?)\}", rf"\1, {name}State}}", window, count=1)
    if count != 1:
        sys.exit("State export not found")
    window = insert_after_last(
        window,
        rf'^\s+\{{ text: @tr\(.*group: @tr\("{re.escape(group)}"\).*\}},\n',
        f'                {{ text: @tr("{menu}"), group: @tr("{group}"), tags: "{tags}", image: IconSet.{icon}, page: AppPages.{name} }},\n',
        f'menu group "{group}" (it must already exist)',
    )
    window = insert_after_last(
        window,
        r"^\s+if current-page == AppPages\.\w+:\s+\w+Page \{ \}\n",
        f"            if current-page == AppPages.{name}: {name}Page {{ }}\n",
        "page switch",
    )
    write("ui/app-window.slint", window)

    # Global state.
    globals_slint = read("ui/globals.slint")
    write(
        "ui/globals.slint",
        globals_slint.rstrip("\n")
        + f"""

export global {name}State {{
    in-out property <string> input;
    in-out property <string> output;
    callback convert();
    callback copy-output();
}}
""",
    )

    # Skeletons.
    write(
        f"ui/pages/{slug}.slint",
        f"""import {{ Button }} from "std-widgets.slint";
import {{ PageContainer }} from "../components/page-container.slint";
import {{ AdvancedTextEdit }} from "../components/advanced-text-edit.slint";
import {{ {name}State }} from "../globals.slint";
import {{ Theme }} from "../theme.slint";

export component {name}Page inherits PageContainer {{
    title-text: @tr("{menu}");
    subtitle-text: @tr("TODO");

    AdvancedTextEdit {{
        text <=> {name}State.input;
        font-family: "monospace";
        font-size: Theme.font-sm;
        vertical-stretch: 1;
        edited(_) => {{
            {name}State.convert();
        }}
    }}

    AdvancedTextEdit {{
        text: {name}State.output;
        read-only: true;
        font-family: "monospace";
        font-size: Theme.font-sm;
        vertical-stretch: 1;
    }}

    Button {{
        text: @tr("Copy");
        enabled: !{name}State.output.is-empty;
        clicked => {{
            {name}State.copy-output();
        }}
    }}
}}
""",
    )
    write(
        f"src/pages/{module}.rs",
        f"""// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{{ComponentHandle, ToSharedString}};

use crate::{{AppWindow, {name}State}};

pub fn register(app: &AppWindow) {{
    let handle = app.as_weak();
    app.global::<{name}State>().on_convert(move || {{
        let ui = handle.unwrap();
        let state = ui.global::<{name}State>();
        let output = crate::units::{module}::convert(state.get_input().as_str());
        state.set_output(output.to_shared_string());
    }});

    let handle = app.as_weak();
    app.global::<{name}State>().on_copy_output(move || {{
        let ui = handle.unwrap();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {{
            let _ = clipboard.set_text(ui.global::<{name}State>().get_output().as_str());
        }}
    }});
}}
""",
    )
    write(
        f"src/units/{module}.rs",
        f"""// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

pub fn convert(input: &str) -> String {{
    input.to_string()
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn convert_returns_input() {{
        assert_eq!(convert("x"), "x");
    }}
}}
""",
    )

    for path, text in STAGED.items():
        (ROOT / path).write_text(text)

    print(f"Added {name} (page index {index}). Next: implement units/{module}.rs and run rust.py verify.")


if __name__ == "__main__":
    main()
