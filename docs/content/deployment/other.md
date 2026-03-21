---
title: "Other Deployment Platforms"
---

# Deploying to Other Platforms

Mythic generates static HTML that works on any hosting platform. This guide covers Netlify, Vercel, and Cloudflare Pages.

## Netlify

### Configuration

Create a `netlify.toml` at your project root:

```toml
[build]
  command = "curl -fsSL https://mythic.site/install.sh | sh && mythic build"
  publish = "public"

[build.environment]
  MYTHIC_VERSION = "0.8.0"

# Redirect rules
[[redirects]]
  from = "/old-path/*"
  to = "/new-path/:splat"
  status = 301

# Custom headers
[[headers]]
  for = "/*"
  [headers.values]
    X-Frame-Options = "DENY"
    X-Content-Type-Options = "nosniff"

[[headers]]
  for = "/css/*"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
  for = "/js/*"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"
```

### Setup Steps

1. Push your Mythic project to a Git repository (GitHub, GitLab, or Bitbucket)
2. Log in to [Netlify](https://app.netlify.com)
3. Click **New site from Git** and select your repository
4. Netlify detects the `netlify.toml` configuration automatically
5. Click **Deploy site**

### Environment Variables

Set `MYTHIC_BASE_URL` in Netlify's site settings under **Build & deploy** > **Environment**:

```
MYTHIC_BASE_URL=https://your-site.netlify.app
```

For custom domains, update this after configuring your domain.

### Deploy Previews

Netlify automatically builds deploy previews for pull requests. Override the base URL for previews:

```toml
[context.deploy-preview]
  command = "curl -fsSL https://mythic.site/install.sh | sh && mythic build"

[context.deploy-preview.environment]
  MYTHIC_BASE_URL = ""
```

Setting an empty base URL makes links relative, which works for any preview domain.

## Vercel

### Configuration

Create a `vercel.json` at your project root:

```json
{
  "buildCommand": "curl -fsSL https://mythic.site/install.sh | sh && mythic build",
  "outputDirectory": "public",
  "framework": null,
  "headers": [
    {
      "source": "/css/(.*)",
      "headers": [
        {
          "key": "Cache-Control",
          "value": "public, max-age=31536000, immutable"
        }
      ]
    },
    {
      "source": "/js/(.*)",
      "headers": [
        {
          "key": "Cache-Control",
          "value": "public, max-age=31536000, immutable"
        }
      ]
    }
  ],
  "redirects": [
    {
      "source": "/old-path/:path*",
      "destination": "/new-path/:path*",
      "permanent": true
    }
  ],
  "trailingSlash": true
}
```

### Setup Steps

1. Push your Mythic project to GitHub, GitLab, or Bitbucket
2. Log in to [Vercel](https://vercel.com)
3. Click **New Project** and import your repository
4. Vercel detects the `vercel.json` configuration
5. Set the **Framework Preset** to **Other**
6. Click **Deploy**

### Environment Variables

In the Vercel dashboard, go to **Settings** > **Environment Variables** and add:

```
MYTHIC_BASE_URL=https://your-site.vercel.app
```

### Using the Vercel CLI

You can also deploy from the command line:

```bash
npm i -g vercel
vercel
```

For production deployments:

```bash
vercel --prod
```

## Cloudflare Pages

### Dashboard Setup

1. Log in to the [Cloudflare dashboard](https://dash.cloudflare.com)
2. Go to **Workers & Pages** > **Create application** > **Pages**
3. Connect your Git repository
4. Configure the build:
   - **Build command:** `curl -fsSL https://mythic.site/install.sh | sh && mythic build`
   - **Build output directory:** `public`
5. Click **Save and Deploy**

### Environment Variables

Add environment variables in **Settings** > **Environment variables**:

```
MYTHIC_BASE_URL=https://your-site.pages.dev
```

### Custom Domain

1. In the Cloudflare Pages project, go to **Custom domains**
2. Click **Set up a custom domain**
3. Enter your domain and follow the DNS instructions

If your domain is already on Cloudflare, DNS records are configured automatically.

### wrangler CLI

Deploy with the Cloudflare `wrangler` CLI:

```bash
npm i -g wrangler

# Build the site first
mythic build

# Deploy
wrangler pages deploy public --project-name=my-site
```

### Headers and Redirects

Cloudflare Pages uses `_headers` and `_redirects` files in the output directory. Place them in your `static/` directory:

```
# static/_headers
/*
  X-Frame-Options: DENY
  X-Content-Type-Options: nosniff

/css/*
  Cache-Control: public, max-age=31536000, immutable

/js/*
  Cache-Control: public, max-age=31536000, immutable
```

```
# static/_redirects
/old-path/* /new-path/:splat 301
```

## General Tips

### Base URL

Always set the correct `base_url` for your deployment target. You can use environment variables to avoid hard-coding it:

```bash
MYTHIC_BASE_URL="https://production.example.com" mythic build
```

### Cache Headers

Since Mythic uses content hashing for CSS and JS files, set long cache times for those assets. The content hash ensures browsers fetch new files when content changes.

### Trailing Slashes

Mythic generates pages with trailing slashes by default (`/about/` not `/about`). Most platforms handle this correctly. If you encounter issues, check the platform's trailing slash configuration.

### 404 Pages

Create a custom 404 page at `content/404.md`:

```yaml
---
title: "Page Not Found"
layout: error
---

# Page Not Found

The page you are looking for does not exist.

[Go to the homepage](/)
```

Most platforms automatically serve `404.html` from the root of your site for missing routes.
