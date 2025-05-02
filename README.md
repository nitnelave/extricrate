# Extricrate

<p align="center">
  <a href="https://github.com/nitnelave/extricrate/actions/workflows/rust.yml?query=branch%3Amaster">
    <img
      src="https://github.com/nitnelave/extricrate/actions/workflows/rust.yml/badge.svg"
      alt="Build"/>
  </a>
  <a href="https://discord.gg/Dned3QwJe2">
    <img alt="Discord" src="https://img.shields.io/discord/898492935446876200?label=discord&logo=discord" />
  </a>
  <a href="https://app.codecov.io/gh/nitnelave/extricrate">
    <img alt="Codecov" src="https://img.shields.io/codecov/c/github/nitnelave/extricrate" />
  </a>
</p>

Automated refactoring tool to extract a Rust module into a separate crate.

> extricate (v): free something from a constraint.

*** WIP, not ready yet ***

## Usage

`cargo extricrate extract --module my_crate.auth --crate_name my_crate_auth`

`cargo extricrate list_dependencies --module my_crate.auth`

## Contributing

This project is an experiment in crowdsourcing open-source software. See
[Contributing](CONTRIBUTING.md).
