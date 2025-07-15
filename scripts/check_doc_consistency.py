#!/usr/bin/env python3
"""
Documentation Consistency Checker for Sage Agent
中英文文档一致性检查器

This script checks for consistency between English and Chinese documentation.
该脚本检查中英文文档之间的一致性。
"""

import os
import sys
import re
import hashlib
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Set, Tuple
import json

class DocConsistencyChecker:
    def __init__(self, root_dir: str = "."):
        self.root_dir = Path(root_dir)
        self.docs_dir = self.root_dir / "docs"
        self.readme_files = ["README.md", "README_zh.md"]
        self.report = {
            "timestamp": datetime.now().isoformat(),
            "status": "unknown",
            "issues": [],
            "stats": {}
        }
    
    def get_file_pairs(self) -> List[Tuple[Path, Path]]:
        """Get pairs of English and Chinese documentation files"""
        pairs = []
        
        # Check main README files
        en_readme = self.root_dir / "README.md"
        zh_readme = self.root_dir / "README_zh.md"
        if en_readme.exists() and zh_readme.exists():
            pairs.append((en_readme, zh_readme))
        
        # Check docs directory structure
        if self.docs_dir.exists():
            for en_file in self.docs_dir.rglob("*.md"):
                if "_zh.md" in en_file.name:
                    continue
                
                # Look for corresponding Chinese file
                zh_file = en_file.parent / f"{en_file.stem}_zh.md"
                if zh_file.exists():
                    pairs.append((en_file, zh_file))
        
        return pairs
    
    def check_file_existence(self) -> Dict:
        """Check if all English files have Chinese counterparts and vice versa"""
        issues = []
        
        # Check main README files
        en_readme = self.root_dir / "README.md"
        zh_readme = self.root_dir / "README_zh.md"
        
        if en_readme.exists() and not zh_readme.exists():
            issues.append({
                "type": "missing_chinese",
                "file": "README_zh.md",
                "message": "Chinese README is missing"
            })
        
        if zh_readme.exists() and not en_readme.exists():
            issues.append({
                "type": "missing_english",
                "file": "README.md",
                "message": "English README is missing"
            })
        
        # Check docs directory
        if self.docs_dir.exists():
            for md_file in self.docs_dir.rglob("*.md"):
                if "_zh.md" in md_file.name:
                    # This is a Chinese file, check for English counterpart
                    en_file = md_file.parent / md_file.name.replace("_zh.md", ".md")
                    if not en_file.exists():
                        issues.append({
                            "type": "missing_english",
                            "file": str(en_file.relative_to(self.root_dir)),
                            "message": f"English counterpart missing for {md_file.relative_to(self.root_dir)}"
                        })
                else:
                    # This is an English file, check for Chinese counterpart
                    zh_file = md_file.parent / f"{md_file.stem}_zh.md"
                    if not zh_file.exists():
                        issues.append({
                            "type": "missing_chinese",
                            "file": str(zh_file.relative_to(self.root_dir)),
                            "message": f"Chinese counterpart missing for {md_file.relative_to(self.root_dir)}"
                        })
        
        return {
            "issues": issues,
            "total_missing": len(issues)
        }
    
    def check_content_freshness(self) -> Dict:
        """Check if files are up-to-date based on modification time"""
        issues = []
        file_pairs = self.get_file_pairs()
        
        for en_file, zh_file in file_pairs:
            en_mtime = en_file.stat().st_mtime
            zh_mtime = zh_file.stat().st_mtime
            
            # If English file is newer than Chinese by more than 1 day
            if en_mtime - zh_mtime > 86400:  # 24 hours in seconds
                issues.append({
                    "type": "outdated_chinese",
                    "en_file": str(en_file.relative_to(self.root_dir)),
                    "zh_file": str(zh_file.relative_to(self.root_dir)),
                    "en_modified": datetime.fromtimestamp(en_mtime).isoformat(),
                    "zh_modified": datetime.fromtimestamp(zh_mtime).isoformat(),
                    "message": f"Chinese file may be outdated"
                })
        
        return {
            "issues": issues,
            "total_outdated": len(issues)
        }
    
    def check_structure_consistency(self) -> Dict:
        """Check if document structure is consistent between languages"""
        issues = []
        file_pairs = self.get_file_pairs()
        
        for en_file, zh_file in file_pairs:
            try:
                with open(en_file, 'r', encoding='utf-8') as f:
                    en_content = f.read()
                with open(zh_file, 'r', encoding='utf-8') as f:
                    zh_content = f.read()
                
                # Extract headers
                en_headers = re.findall(r'^#+\s+(.+)$', en_content, re.MULTILINE)
                zh_headers = re.findall(r'^#+\s+(.+)$', zh_content, re.MULTILINE)
                
                # Check if header count matches
                if len(en_headers) != len(zh_headers):
                    issues.append({
                        "type": "structure_mismatch",
                        "en_file": str(en_file.relative_to(self.root_dir)),
                        "zh_file": str(zh_file.relative_to(self.root_dir)),
                        "en_headers": len(en_headers),
                        "zh_headers": len(zh_headers),
                        "message": f"Header count mismatch: EN={len(en_headers)}, ZH={len(zh_headers)}"
                    })
                
                # Check for code blocks consistency
                en_code_blocks = len(re.findall(r'```', en_content))
                zh_code_blocks = len(re.findall(r'```', zh_content))
                
                if en_code_blocks != zh_code_blocks:
                    issues.append({
                        "type": "code_block_mismatch",
                        "en_file": str(en_file.relative_to(self.root_dir)),
                        "zh_file": str(zh_file.relative_to(self.root_dir)),
                        "en_blocks": en_code_blocks // 2,  # Divide by 2 for pairs
                        "zh_blocks": zh_code_blocks // 2,
                        "message": f"Code block count mismatch"
                    })
                
            except Exception as e:
                issues.append({
                    "type": "read_error",
                    "files": [str(en_file.relative_to(self.root_dir)), str(zh_file.relative_to(self.root_dir))],
                    "error": str(e),
                    "message": f"Error reading files: {e}"
                })
        
        return {
            "issues": issues,
            "total_structure_issues": len(issues)
        }
    
    def generate_report(self) -> Dict:
        """Generate comprehensive consistency report"""
        print("🔍 Checking documentation consistency...")
        
        # Run all checks
        existence_check = self.check_file_existence()
        freshness_check = self.check_content_freshness()
        structure_check = self.check_structure_consistency()
        
        # Compile report
        all_issues = (
            existence_check["issues"] + 
            freshness_check["issues"] + 
            structure_check["issues"]
        )
        
        self.report.update({
            "status": "pass" if len(all_issues) == 0 else "fail",
            "issues": all_issues,
            "stats": {
                "total_issues": len(all_issues),
                "missing_files": existence_check["total_missing"],
                "outdated_files": freshness_check["total_outdated"],
                "structure_issues": structure_check["total_structure_issues"],
                "file_pairs_checked": len(self.get_file_pairs())
            }
        })
        
        return self.report
    
    def print_report(self, report: Dict):
        """Print human-readable report"""
        print(f"\n📊 Documentation Consistency Report")
        print(f"Generated: {report['timestamp']}")
        print(f"Status: {'✅ PASS' if report['status'] == 'pass' else '❌ FAIL'}")
        print(f"File pairs checked: {report['stats']['file_pairs_checked']}")
        print(f"Total issues: {report['stats']['total_issues']}")
        
        if report['stats']['total_issues'] > 0:
            print(f"\n🔍 Issues Found:")
            print(f"  • Missing files: {report['stats']['missing_files']}")
            print(f"  • Outdated files: {report['stats']['outdated_files']}")
            print(f"  • Structure issues: {report['stats']['structure_issues']}")
            
            print(f"\n📋 Detailed Issues:")
            for i, issue in enumerate(report['issues'], 1):
                print(f"  {i}. [{issue['type'].upper()}] {issue['message']}")
                if 'file' in issue:
                    print(f"     File: {issue['file']}")
                if 'en_file' in issue and 'zh_file' in issue:
                    print(f"     EN: {issue['en_file']}")
                    print(f"     ZH: {issue['zh_file']}")
        else:
            print(f"\n✅ All documentation files are consistent!")
    
    def save_report(self, filename: str = "doc_consistency_report.json"):
        """Save report to JSON file"""
        report_path = self.root_dir / filename
        with open(report_path, 'w', encoding='utf-8') as f:
            json.dump(self.report, f, indent=2, ensure_ascii=False)
        print(f"📄 Report saved to: {report_path}")

def main():
    """Main function"""
    checker = DocConsistencyChecker()
    report = checker.generate_report()
    checker.print_report(report)
    checker.save_report()
    
    # Exit with error code if issues found
    sys.exit(0 if report['status'] == 'pass' else 1)

if __name__ == "__main__":
    main()
