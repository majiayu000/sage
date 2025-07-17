#!/usr/bin/env python3

import re
import os

def add_default_impl(file_path, struct_name):
    """Add Default implementation for a struct"""
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Find the impl block for the struct
    impl_pattern = rf'impl {struct_name} {{\s*pub fn new\(\) -> Self {{\s*Self[^}}]*}}\s*}}'
    match = re.search(impl_pattern, content, re.DOTALL)
    
    if match:
        # Insert Default impl before the existing impl
        default_impl = f"""impl Default for {struct_name} {{
    fn default() -> Self {{
        Self::new()
    }}
}}

"""
        # Insert before the existing impl
        new_content = content[:match.start()] + default_impl + content[match.start():]
        
        with open(file_path, 'w') as f:
            f.write(new_content)
        print(f"Added Default impl for {struct_name} in {file_path}")

def fix_redundant_closures(file_path):
    """Fix redundant closures like |e| ToolError::Io(e)"""
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Replace redundant closures
    content = re.sub(r'\.map_err\(\|e\| ToolError::Io\(e\)\)', '.map_err(ToolError::Io)', content)
    content = re.sub(r'\.map_err\(\|e\| ToolError::Json\(e\)\)', '.map_err(ToolError::Json)', content)
    
    with open(file_path, 'w') as f:
        f.write(content)
    print(f"Fixed redundant closures in {file_path}")

def fix_needless_borrow(file_path):
    """Fix needless borrows"""
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Fix specific needless borrows
    content = re.sub(r'&\["/C", "start", &url\]', '["/C", "start", &url]', content)
    content = re.sub(r'&file_path', 'file_path', content)
    
    with open(file_path, 'w') as f:
        f.write(content)
    print(f"Fixed needless borrows in {file_path}")

def fix_or_insert_with(file_path):
    """Fix or_insert_with to or_default"""
    with open(file_path, 'r') as f:
        content = f.read()
    
    content = re.sub(r'\.or_insert_with\(Vec::new\)', '.or_default()', content)
    
    with open(file_path, 'w') as f:
        f.write(content)
    print(f"Fixed or_insert_with in {file_path}")

# List of structs that need Default implementations
structs_to_fix = [
    ("src/tools/task_mgmt/task_management.rs", "TaskList"),
    ("src/tools/task_mgmt/task_management.rs", "ViewTasklistTool"),
    ("src/tools/task_mgmt/task_management.rs", "AddTasksTool"),
    ("src/tools/task_mgmt/task_management.rs", "UpdateTasksTool"),
    ("src/tools/task_mgmt/reorganize_tasklist.rs", "ReorganizeTasklistTool"),
    ("src/tools/network/web_search.rs", "WebSearchTool"),
    ("src/tools/network/web_fetch.rs", "WebFetchTool"),
    ("src/tools/network/browser.rs", "BrowserTool"),
    ("src/tools/diagnostics/ide_diagnostics.rs", "DiagnosticsTool"),
    ("src/tools/diagnostics/content_processing.rs", "ViewRangeUntruncatedTool"),
    ("src/tools/diagnostics/content_processing.rs", "SearchUntruncatedTool"),
    ("src/tools/diagnostics/memory.rs", "RememberTool"),
    ("src/tools/diagnostics/mermaid.rs", "RenderMermaidTool"),
]

# Files that need redundant closure fixes
files_with_closures = [
    "src/tools/file_ops/json_edit.rs",
    "src/tools/file_ops/codebase_retrieval.rs",
]

# Files that need needless borrow fixes
files_with_borrows = [
    "src/tools/network/browser.rs",
    "src/tools/file_ops/codebase_retrieval.rs",
]

# Files that need or_insert_with fixes
files_with_or_insert = [
    "src/tools/file_ops/codebase_retrieval.rs",
]

if __name__ == "__main__":
    # Add Default implementations
    for file_path, struct_name in structs_to_fix:
        if os.path.exists(file_path):
            add_default_impl(file_path, struct_name)
    
    # Fix redundant closures
    for file_path in files_with_closures:
        if os.path.exists(file_path):
            fix_redundant_closures(file_path)
    
    # Fix needless borrows
    for file_path in files_with_borrows:
        if os.path.exists(file_path):
            fix_needless_borrow(file_path)
    
    # Fix or_insert_with
    for file_path in files_with_or_insert:
        if os.path.exists(file_path):
            fix_or_insert_with(file_path)
    
    print("All clippy fixes applied!")
