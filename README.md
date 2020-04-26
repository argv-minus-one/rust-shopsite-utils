# rust-shopsite-utils

This project contains a few utilities, written in [Rust](https://rust-lang.org/), to help with maintaining an instance of the [ShopSite](https://www.shopsite.com/) web product catalog and shopping cart software.

This project is MIT licensed. ShopSite itself is a proprietary commercial product.

**This project is not affiliated with or endorsed by ShopSite, Inc.**

## Contents

There are three packages in this project:

* `shopsite-aa`: A `Deserializer` for ShopSite's `.aa` files, for use with the [Serde](https://serde.rs/) library.
* `shopsite-aa2json`: A command-line tool that translates a ShopSite `.aa` file to JSON, using the `shopsite-aa` library.
* `make-shopsite-backup`: (Not written yet.) Generates a backup of a ShopSite store. Safely dumps the SQLite databases.
