# ICO Builder
A crate for creating multi-size ICO files from separate images.
The images are automatically resized to the specified sizes.

## Usage
This example creates an ICO with the [bare minimum] icon sizes: 16x16, 24x24, 32x32, 48x48, 256x256.
For each size, the closest source file is chosen and resized as needed.

```rust
use ico_builder::IcoBuilder;

IcoBuilder::default()
  .add_source_file("app-icon-32x32.png")
  .add_source_file("app-icon-256x256.png")
  .build_file("app-icon.ico");
```


[bare minimum]: https://learn.microsoft.com/en-us/windows/apps/design/style/iconography/app-icon-construction#icon-scaling

## [Docs](https://docs.rs/ico-builder)

## License
Licensed under either of

* Apache License, Version 2.0
  ([license-apache.txt](license-apache.txt) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([license-mit.txt](license-mit.txt) or http://opensource.org/licenses/MIT)

at your option.

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
