# Development Instructions

## Create a Release

Change the version number in `cargo.toml`

```bash
git commit -am "release: {VERSION}"
git tag "v{VERSION}"
git push
git push --tags
```
