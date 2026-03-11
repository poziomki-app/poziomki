<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./assets/dark-mode.svg">
    <source media="(prefers-color-scheme: light)" srcset="./assets/light-mode.svg">
    <img alt="poziomki" src="./assets/light-mode.svg" width="500">
  </picture>
</p>

<sub><a href="./README.md">pl</a> · <b>en</b></sub>

a social app for university students, connecting people by shared interests and encouraging spending more time together through local events

## operations

the backend metrics dashboard lives at `/api/v1/metrics/` and the JSON API at `/api/v1/metrics`

- set `OPS_STATUS_TOKEN` to enable both routes
- the JSON API expects the token in the `x-ops-token` header
- the dashboard expects the token in the `token` query parameter
- TimescaleDB is optional; when metrics samples cannot be loaded from the database, the backend falls back to in-memory series and marks the response as degraded

## license

this project is available under AGPLv3 license

## funding

<p align="left">
  <img src="./assets/funding.png" />
</p>
