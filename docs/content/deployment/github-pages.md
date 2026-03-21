---
title: "GitHub Pages"
---

# Deploying to GitHub Pages

Mythic provides an official GitHub Action that builds your site and deploys it to GitHub Pages automatically on every push.

## Using the Mythic GitHub Action

Create a workflow file at `.github/workflows/deploy.yml`:

```yaml
name: Deploy to GitHub Pages

on:
  push:
    branches: [main]

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Build with Mythic
        uses: mythic-ssg/mythic-action@v1
        with:
          version: "latest"

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: public

      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

### Action Inputs

The `mythic-ssg/mythic-action@v1` action supports these inputs:

| Input       | Default    | Description                           |
|-------------|------------|---------------------------------------|
| `version`   | `"latest"` | Mythic version to install            |
| `args`      | `""`       | Additional arguments for `mythic build`|

Example with a specific version and arguments:

```yaml
- name: Build with Mythic
  uses: mythic-ssg/mythic-action@v1
  with:
    version: "0.8.0"
    args: "--base-url https://myuser.github.io/my-repo"
```

## Repository Setup

### Enable GitHub Pages

1. Go to your repository on GitHub
2. Navigate to **Settings** > **Pages**
3. Under **Source**, select **GitHub Actions**
4. Save

### Base URL for Project Sites

If your site is hosted at `https://username.github.io/repo-name/` (a project site rather than a user site), you need to set the base URL:

```toml
# mythic.toml
[site]
base_url = "https://username.github.io/repo-name"
```

Or override it in the workflow:

```yaml
- name: Build with Mythic
  uses: mythic-ssg/mythic-action@v1
  with:
    args: "--base-url https://username.github.io/repo-name"
```

## Custom Domains

### Setting Up a Custom Domain

1. In your repository, go to **Settings** > **Pages**
2. Enter your custom domain under **Custom domain**
3. Click **Save**

4. Add a `CNAME` file to your `static/` directory so it is included in every build:

```
static/CNAME
```

Contents of the file (no trailing newline):

```
www.example.com
```

5. Configure DNS with your domain registrar:

For apex domains (`example.com`), add A records:

```
185.199.108.153
185.199.109.153
185.199.110.153
185.199.111.153
```

For subdomains (`www.example.com`), add a CNAME record:

```
www.example.com -> username.github.io
```

6. Enable **Enforce HTTPS** in GitHub Pages settings once DNS propagates.

7. Update your `mythic.toml`:

```toml
[site]
base_url = "https://www.example.com"
```

## Manual Build Workflow

If you prefer to install Mythic manually instead of using the action:

```yaml
name: Deploy to GitHub Pages

on:
  push:
    branches: [main]

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Mythic
        run: |
          curl -fsSL https://mythic.site/install.sh | sh

      - name: Build site
        run: mythic build

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: public

      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

## Preview Deployments for Pull Requests

Deploy preview builds for pull requests using a separate workflow:

```yaml
name: PR Preview

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  preview:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build with Mythic
        uses: mythic-ssg/mythic-action@v1
        with:
          args: "--drafts --base-url https://preview.example.com/pr-${{ github.event.number }}"

      # Add your preview deployment step here (e.g., Netlify, Surge, etc.)
```

## Caching

The Mythic GitHub Action automatically caches the Mythic binary between runs. For manual workflows, add caching:

```yaml
- name: Cache Mythic binary
  uses: actions/cache@v4
  with:
    path: ~/.local/bin/mythic
    key: mythic-${{ runner.os }}-0.8.0
```

## Build Status Badge

Add a build status badge to your README:

```markdown
[![Deploy](https://github.com/username/repo/actions/workflows/deploy.yml/badge.svg)](https://github.com/username/repo/actions/workflows/deploy.yml)
```
