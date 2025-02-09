# Manderrow

[![build](https://github.com/Jack-the-Pro101/manderrow/actions/workflows/build.yml/badge.svg)](https://github.com/Jack-the-Pro101/manderrow/actions/workflows/build.yml)

- [About](#about)
- [Features and Improvements](#features-and-improvements)
- [Contributing](#contributing)

## About

Manderrow is a mod manager with the goal of becoming the successor to [r2modman](https://github.com/ebkr/r2modmanPlus). Because of this, it will support all of the existing features of r2modman, in addition to more. Also, Manderrow aims to address the major issues of r2modman, outlined in the next section.

## Features and Improvements

### Improvements:

| r2modman                                                                                                                                                      | Manderrow                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Performance of loading mod list is extremely slow (20-30s)                                                                                                    | Optimized JSON parsing results in 3-8x faster loading speeds                                   |
| Memory usage while loading mod list is extremely high (peaks around 1GB)                                                                                      | Optimized JSON parsing results in around half the memory usage                                 |
| No multi-language support after 5 years, despite having an i18n library installed                                                                             | Multiple languages (English, Spanish, French, Mandarin) will be supported on the first release |
| Unintuitive UI choices (e.g. profile switching and sharing is in settings menu)                                                                               | Intuitive UI                                                                                   |
| Auto-updater not functional (this issue is acknowledged in the [r2modman docs](https://r2modman.net/how-to-update-r2modman#h.3znysh7) yet has not been fixed) | Functional auto-updater                                                                        |
| Uses deprecated and old technologies: Vue 2, moment JS, Electron v24                                                                                          | Uses modern and up-to-date technologies: Solid JS, JS Intl API, Tauri v2                       |

### New features:

- Customizable themes
- Better UX: more cancel control, background-processed UI instead of fully blocking UI, and modernized interface.
- (might add eventually) Ability to support new games without having to update the app if the game uses existing launchers.

## Contributing

This project is under rapid active development, and is not yet in release. If you wish you help, there are todo lists in the issues tab, or if you are interested in getting in contact, you can friend me on Discord `@emperor_of_bluegaria`.
