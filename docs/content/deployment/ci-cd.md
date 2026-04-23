---
title: "CI/CD Integration"
---

# CI/CD Integration

Mythic provides several features designed for continuous integration and deployment pipelines: structured build output, quiet mode, deploy manifests, and content diffing.

## Structured Build Output

Use the `--json` flag to get machine-readable build output:

```bash
mythic build --json
```

This prints a JSON object to stdout with build statistics:

```json
{
  "total_pages": 42,
  "pages_written": 38,
  "pages_unchanged": 4,
  "pages_skipped": 2,
  "elapsed_ms": 156,
  "lint_warnings": 1,
  "errors": 0
}
```

You can parse this output in your CI scripts to make decisions based on build results:

```bash
result=$(mythic build --json)
errors=$(echo "$result" | jq '.errors')
if [ "$errors" -gt 0 ]; then
  echo "Build had errors, aborting deploy"
  exit 1
fi
```

## Quiet Mode

Use `--quiet` to suppress all output except errors. This keeps CI logs clean:

```bash
mythic build --quiet
```

In quiet mode, the build produces no output on success. Only errors are printed to stderr. This is useful when you only care about pass/fail status.

## Deploy Manifest

When Mythic builds your site, it generates a `deploy-manifest.json` file in the output directory. This manifest lists every file that was written, along with its content hash:

```json
{
  "files": [
    { "path": "index.html", "hash": "a1b2c3d4" },
    { "path": "blog/my-post/index.html", "hash": "e5f6a7b8" },
    { "path": "css/main.a1b2c3d4.css", "hash": "a1b2c3d4" },
    { "path": "images/photo.640.webp", "hash": "9c8d7e6f" }
  ]
}
```

Use the deploy manifest to implement minimal deployments that only upload files whose hashes have changed since the last deploy. This dramatically reduces deployment time for large sites.

## Content Diffing

Mythic tracks which pages were added, modified, or removed between builds. This information is included in the JSON build output:

```bash
mythic build --json
```

```json
{
  "total_pages": 42,
  "pages_written": 38,
  "pages_unchanged": 4,
  "pages_skipped": 2,
  "elapsed_ms": 156,
  "diff": {
    "added": ["blog/new-post/index.html"],
    "modified": ["blog/updated-post/index.html", "index.html"],
    "removed": ["blog/old-post/index.html"]
  }
}
```

The diff information is useful for cache invalidation, selective deployment, and generating changelogs.

## Example: GitHub Actions Deploy-Only-Changed

The following GitHub Actions workflow builds the site and deploys only the files that changed. It uses the deploy manifest to determine which files need to be uploaded.

```yaml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Mythic
        run: curl -fsSL https://mythic.site/install.sh | sh

      - name: Build site
        run: mythic build --json > build-result.json

      - name: Check for errors
        run: |
          errors=$(jq '.errors' build-result.json)
          if [ "$errors" -gt 0 ]; then
            echo "Build failed with $errors errors"
            exit 1
          fi

      - name: Deploy changed files
        run: |
          # Read the diff from the build output
          added=$(jq -r '.diff.added[]' build-result.json 2>/dev/null)
          modified=$(jq -r '.diff.modified[]' build-result.json 2>/dev/null)

          # Combine added and modified files
          changed_files=$(echo -e "$added\n$modified" | sort -u | grep -v '^$')

          if [ -z "$changed_files" ]; then
            echo "No files changed, skipping deploy"
            exit 0
          fi

          echo "Deploying $(echo "$changed_files" | wc -l | tr -d ' ') changed files"

          # Upload only changed files to your hosting provider
          echo "$changed_files" | while read -r file; do
            echo "  Uploading: $file"
            # Replace with your deploy command, e.g.:
            # aws s3 cp "public/$file" "s3://my-bucket/$file"
          done

      - name: Invalidate cache for removed files
        run: |
          removed=$(jq -r '.diff.removed[]' build-result.json 2>/dev/null)
          if [ -n "$removed" ]; then
            echo "Invalidating removed files:"
            echo "$removed"
            # Replace with your cache invalidation command
          fi
```

## Example: GitLab CI

```yaml
build-and-deploy:
  image: ubuntu:latest
  script:
    - curl -fsSL https://mythic.site/install.sh | sh
    - mythic build --quiet
    - mythic check
  artifacts:
    paths:
      - public/
  only:
    - main
```

## Tips for CI/CD

- Use `mythic build --json` when you need to inspect build results programmatically.
- Use `mythic build --quiet` when you only need pass/fail status.
- Run `mythic check` after the build to catch accessibility and link issues before deployment.
- Store the `deploy-manifest.json` as a build artifact so you can compare it across deployments.
- Set `MYTHIC_BASE_URL` as an environment variable to override the base URL for staging or preview environments:

```bash
MYTHIC_BASE_URL="https://preview.example.com" mythic build
```
