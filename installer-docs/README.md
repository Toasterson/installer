# illumos Installer Documentation

This repository contains the documentation for the illumos Installer, built using [mdBook](https://rust-lang.github.io/mdBook/).

## Structure

The documentation is organized into the following sections:

- **Machine Configuration**: Documentation for the machineconfig component, which defines the overall machine configuration.
- **System Configuration**: Documentation for the sysconfig component, which manages system settings.
- **Appendix**: Additional resources, including a glossary and troubleshooting guide.

## Building the Documentation

To build the documentation locally, you need to have mdBook installed. If you don't have it installed, you can install it using Cargo:

```bash
cargo install mdbook
```

Once mdBook is installed, you can build the documentation by running:

```bash
cd installer-docs
mdbook build
```

This will generate the HTML documentation in the `book` directory.

To serve the documentation locally and automatically rebuild it when changes are made, you can use:

```bash
cd installer-docs
mdbook serve
```

This will start a local web server at http://localhost:3000 where you can view the documentation.

## Hosting the Documentation Online

The documentation can be hosted online using various services. Here are instructions for some common options:

### GitHub Pages

To host the documentation on GitHub Pages:

1. Build the documentation:

   ```bash
   cd installer-docs
   mdbook build
   ```

2. Create a new branch for the GitHub Pages site:

   ```bash
   git checkout -b gh-pages
   ```

3. Copy the contents of the `book` directory to the root of the repository:

   ```bash
   cp -r book/* .
   ```

4. Add, commit, and push the changes:

   ```bash
   git add .
   git commit -m "Add documentation for GitHub Pages"
   git push origin gh-pages
   ```

5. Configure GitHub Pages to use the `gh-pages` branch in the repository settings.

### Netlify

To host the documentation on Netlify:

1. Create a `netlify.toml` file in the root of the repository with the following content:

   ```toml
   [build]
   command = "cd installer-docs && mdbook build"
   publish = "installer-docs/book"
   ```

2. Connect your repository to Netlify and configure it to use the settings in the `netlify.toml` file.

### GitLab Pages

To host the documentation on GitLab Pages:

1. Create a `.gitlab-ci.yml` file in the root of the repository with the following content:

   ```yaml
   pages:
     image: rust:latest
     before_script:
       - cargo install mdbook
     script:
       - cd installer-docs && mdbook build
       - mv book ../public
     artifacts:
       paths:
         - public
     only:
       - main
   ```

2. Push the changes to GitLab, and GitLab CI will build and deploy the documentation to GitLab Pages.

## Contributing

Contributions to the documentation are welcome! Please feel free to submit a pull request with your changes.

When contributing, please follow these guidelines:

1. Use clear and concise language.
2. Provide examples where appropriate.
3. Follow the existing structure and formatting.
4. Test your changes by building the documentation locally before submitting a pull request.

## License

The documentation is licensed under the same license as the illumos Installer project.