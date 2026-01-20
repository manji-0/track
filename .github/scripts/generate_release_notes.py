#!/usr/bin/env python3
"""
Generate AI-powered release notes from git diff between versions.
"""
import os
import sys
import json
import subprocess
from urllib import request, error

def get_git_diff(previous_tag, current_tag):
    """Get commits and diff stats between two tags."""
    try:
        if previous_tag:
            # Get commits between tags
            commits_result = subprocess.run(
                ['git', 'log', '--pretty=format:- %s (%h)', f'{previous_tag}..{current_tag}'],
                capture_output=True, text=True, check=True
            )
            # Get diff stats
            diff_result = subprocess.run(
                ['git', 'diff', '--stat', f'{previous_tag}..{current_tag}'],
                capture_output=True, text=True, check=True
            )
        else:
            # First release - get all commits
            commits_result = subprocess.run(
                ['git', 'log', '--pretty=format:- %s (%h)', current_tag],
                capture_output=True, text=True, check=True
            )
            # Get root commit
            root_commit = subprocess.run(
                ['git', 'rev-list', '--max-parents=0', 'HEAD'],
                capture_output=True, text=True, check=True
            ).stdout.strip()
            diff_result = subprocess.run(
                ['git', 'diff', '--stat', root_commit, current_tag],
                capture_output=True, text=True, check=True
            )
        
        return commits_result.stdout, diff_result.stdout
    except subprocess.CalledProcessError as e:
        print(f"Error getting git diff: {e}", file=sys.stderr)
        return "", ""

def generate_ai_notes(commits, diff_stat, current_tag, previous_tag, github_token):
    """Generate release notes using GitHub Models API."""
    prompt = f"""Based on the following git changes for version {current_tag}, generate professional release notes in Japanese.

Include these sections if applicable:
- 新機能 (New Features)
- 改善 (Improvements)
- バグ修正 (Bug Fixes)

Commits:
{commits}

File changes:
{diff_stat}

Format the response as clear, professional release notes in markdown."""

    payload = {
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant that generates professional release notes in Japanese."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "model": "gpt-4o",
        "temperature": 0.7,
        "max_tokens": 1500
    }

    try:
        req = request.Request(
            'https://models.inference.ai.azure.com/chat/completions',
            data=json.dumps(payload).encode('utf-8'),
            headers={
                'Content-Type': 'application/json',
                'Authorization': f'Bearer {github_token}'
            },
            method='POST'
        )
        
        with request.urlopen(req, timeout=30) as response:
            data = json.loads(response.read().decode('utf-8'))
            return data['choices'][0]['message']['content']
    except (error.URLError, error.HTTPError, KeyError, json.JSONDecodeError) as e:
        print(f"AI generation failed: {e}", file=sys.stderr)
        return None

def create_fallback_notes(commits, diff_stat, current_tag, previous_tag):
    """Create structured release notes as fallback."""
    return f"""## {current_tag}

### 📝 変更内容 (Changes)

{commits}

### 📊 変更統計 (Change Statistics)

```
{diff_stat}
```

> バージョン {previous_tag or '初版'} からの変更"""

def main():
    # Get environment variables
    current_tag = os.environ.get('CURRENT_TAG', '')
    previous_tag = os.environ.get('PREVIOUS_TAG', '')
    github_token = os.environ.get('GITHUB_TOKEN', '')
    
    if not current_tag:
        print("Error: CURRENT_TAG environment variable not set", file=sys.stderr)
        sys.exit(1)
    
    print(f"Generating release notes for {current_tag} (previous: {previous_tag or 'none'})", file=sys.stderr)
    
    # Get git diff
    commits, diff_stat = get_git_diff(previous_tag, current_tag)
    
    if not commits:
        print("No commits found", file=sys.stderr)
        sys.exit(1)
    
    # Try to generate AI notes
    notes = None
    if github_token:
        notes = generate_ai_notes(commits, diff_stat, current_tag, previous_tag, github_token)
    
    # Fallback to structured format if AI fails
    if not notes:
        print("Using structured format for release notes", file=sys.stderr)
        notes = create_fallback_notes(commits, diff_stat, current_tag, previous_tag)
    
    # Output notes
    print(notes)

if __name__ == '__main__':
    main()
