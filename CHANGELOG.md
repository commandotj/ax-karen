# Changelog

## [0.12.0](https://github.com/commandotj/ax-karen/compare/v0.11.0...v0.12.0) (2026-10-18)


### Features

* add saved Power Efficiency scan worker presets ([b4e1a22](https://github.com/commandotj/ax-karen/commit/b4e1a22fd997478858c7a455ad287a5f94dd9c1c))
* add saved Power Efficiency scan worker presets (3% faster, 53% less CPU at 8 threads) ([b60372b](https://github.com/commandotj/ax-karen/commit/b60372b73e424466c521fbd3883d6098c027ec70))
* **app:** offer restart as administrator on Windows ([5f374c3](https://github.com/commandotj/ax-karen/commit/5f374c3b471b3569540f9900a30618979856acd2))
* **ci:** setup Publishify CI/CD branch gates, release pipeline, and distribution (RFC 0023) ([6ae48ce](https://github.com/commandotj/ax-karen/commit/6ae48ce9b3931479155da5a803a5a124995f60ce))
* **ci:** setup Publishify CI/CD branch gates, release pipeline, and distribution (RFC 0023) ([1c132be](https://github.com/commandotj/ax-karen/commit/1c132beaeff6d880b07c16baa1c7a764b4136ce7))
* **desktop:** enforce single-instance guard via tauri-plugin-single-instance (RFC 0021) ([de1f4b0](https://github.com/commandotj/ax-karen/commit/de1f4b025eeda0aa40f4f3a39377e22f5482d540))
* **desktop:** rotary drum tab wheel, custom scrollbars, and archive RFCs 0010/0011/0012 ([fc02d1a](https://github.com/commandotj/ax-karen/commit/fc02d1ae478b63fa21a2c6e428ccdcff748a9e99))
* **desktop:** steampunk redesign, Space tab parity, settings and full coverage ([1c8a267](https://github.com/commandotj/ax-karen/commit/1c8a26721f5ca2265641c6dcaff57735422cba3b))
* **i18n:** multi-language internationalization for CJK, EN, FR, DE, ES, RU (RFC 0022) ([3295704](https://github.com/commandotj/ax-karen/commit/3295704b177dcd53e635125a3ad2dec22be75396))
* **network:** implement unprivileged process socket attribution and connection astrolabe (RFC 0020) ([8d41e31](https://github.com/commandotj/ax-karen/commit/8d41e31c40825a2b015b7a59a37d35799296e327))
* **release:** automate develop promotion PR ([9605a2a](https://github.com/commandotj/ax-karen/commit/9605a2aa2f688ba83e72268196d1bc0cc604a079))
* **release:** automate version promotion and publishing ([c1217d8](https://github.com/commandotj/ax-karen/commit/c1217d8383cf683f4f34d550c0463d629e019650))
* **release:** automate version promotion and publishing ([15e1fed](https://github.com/commandotj/ax-karen/commit/15e1fed576a033625cbf60a12a6825345e482a9f))
* **removal:** refuse profile folders and flush after elevated removals ([4d6a65e](https://github.com/commandotj/ax-karen/commit/4d6a65e00c78aa97edad4aa21d2d03bffd8af6ff))
* **scan:** read the NTFS master file table when elevated ([0b1725d](https://github.com/commandotj/ax-karen/commit/0b1725d40e19b2aae7bf9dada9cef48a68173f8a))
* show Power Efficiency as a gauge in the settings row ([eabe10f](https://github.com/commandotj/ax-karen/commit/eabe10f64294b98db8673a6734134c39aacc9c23))
* **tray:** show CPU and Memory side by side in system tray title ([36d9be9](https://github.com/commandotj/ax-karen/commit/36d9be9844eb3c8f1534f3ae266e834c7df95b91))


### Bug Fixes

* allow platform-specific CPU sampler fallback lints ([4e6b15c](https://github.com/commandotj/ax-karen/commit/4e6b15ccfaa42c32406368a5052ff14d1a9387e0))
* **ci:** cross-platform test parity, Linux socket pattern, and dynamic Tauri signing ([67ca8ae](https://github.com/commandotj/ax-karen/commit/67ca8aee085984a2806263c0e321dd28bd49d655))
* **ci:** satisfy clippy float_cmp and complete 100% i18n coverage ([0ededa8](https://github.com/commandotj/ax-karen/commit/0ededa83fd901f2f528a8b9fe1b4dff6b025b626))
* **clippy:** allow float_cmp in tests and make is_dashboard_volume const fn ([a1dbb84](https://github.com/commandotj/ax-karen/commit/a1dbb84d9da3be766f49acd13efd17a8eff70eea))
* keep a check on the saved Power Efficiency preset ([464e476](https://github.com/commandotj/ax-karen/commit/464e476c9e5704c47dcb3d72dae56a14059b5afc))
* keep a checkout's config from running programs during git status ([46f0c8f](https://github.com/commandotj/ax-karen/commit/46f0c8fc2f0a1572e95dff621fb09d0d435ccfbe))
* never delete into a filesystem mounted inside a target ([fdb2e42](https://github.com/commandotj/ax-karen/commit/fdb2e42e5e5414c38cc498befba32b8fa94ea065))
* **release:** correct version selectors and promotion trigger ([051c1be](https://github.com/commandotj/ax-karen/commit/051c1be7e6fd98f046d48cc896fbf9ffc481870a))
* **release:** correct version selectors and promotion trigger ([8aa4135](https://github.com/commandotj/ax-karen/commit/8aa4135e92027216acfcbad7de67a4d471978f86))
* **release:** generate version PRs from workspace package ([ce2691b](https://github.com/commandotj/ax-karen/commit/ce2691b30fa066adc1abc3ea69486969badad156))
* **release:** generate version PRs from workspace package ([7421793](https://github.com/commandotj/ax-karen/commit/7421793c33963fb541af08cab4028b619cfb8c05))
* **release:** separate Apple notarization key and ID ([41aec90](https://github.com/commandotj/ax-karen/commit/41aec904672437548a5cca1a1a67e4e59c377172))
* **release:** separate Apple notarization key and ID ([7089a34](https://github.com/commandotj/ax-karen/commit/7089a347617d9e789c4d3ec0610cc0ac97654fbc))
* **release:** support inherited cargo workspace versions ([6237384](https://github.com/commandotj/ax-karen/commit/6237384b117785703a15fa0cc48a620071ae4ad0))
* **release:** support inherited cargo workspace versions ([e8984dd](https://github.com/commandotj/ax-karen/commit/e8984dd3eaa95577d290b2eac71949089e7c58f8))
* **release:** update shared version files without rust strategy ([a879eb9](https://github.com/commandotj/ax-karen/commit/a879eb9c2edda0846df0aee6436f371744de5a73))
* **release:** use Release Please TOML selector shape ([1de75fa](https://github.com/commandotj/ax-karen/commit/1de75fa37ce3420805bb7ea3154ff84ac336fe4b))
* **space:** exclude foreign-volume marks from free-space projection ([536f037](https://github.com/commandotj/ax-karen/commit/536f037e0fd7acaab026e1726548987d057b9003))
* **space:** exclude foreign-volume marks from free-space projection ([dfd1fdc](https://github.com/commandotj/ax-karen/commit/dfd1fdc649a829088958e583e3962b13b3292a44))
* state the measured Power Efficiency tradeoff in the tooltip ([5b92561](https://github.com/commandotj/ax-karen/commit/5b925614c6a8ee90dfc982426b54072e43f086a9))
* **windows:** embed icon in executable ([816effa](https://github.com/commandotj/ax-karen/commit/816effae97cb806e838e2aa993459b4b82988edd))
* **windows:** open test fixture with write access for set_modified ([e207880](https://github.com/commandotj/ax-karen/commit/e207880efc11dd361f54377b3774f7ba562e1985))
* **windows:** satisfy icon build lint ([358e410](https://github.com/commandotj/ax-karen/commit/358e4104bbc4d3989d63a2c64cefd8b7170f536a))
* **windows:** support Windows drive mount point in volumes_include_the_root_volume test ([c6a4f59](https://github.com/commandotj/ax-karen/commit/c6a4f59f90e458d4176a839b987b9d82cdd4e8e7))
* wire the mouse side buttons to back and forward ([319d0d4](https://github.com/commandotj/ax-karen/commit/319d0d4adab18f81bd531fe0c75d26ba7d276e49))


### Performance Improvements

* bound scan admission and adapt macOS worker budgets ([3021350](https://github.com/commandotj/ax-karen/commit/30213505c2cb7e38ed98fc5f5c3a887cebc16554))
* **classify:** classify without per-directory allocations ([03d3e7a](https://github.com/commandotj/ax-karen/commit/03d3e7a1592e532efd89b6069a9411b20185686e))
* **insights:** skip subtrees too small to list ([d253949](https://github.com/commandotj/ax-karen/commit/d2539490faaa8723791bb8ecd015a85972977462))
* retain direct Rayon dispatch for pool-sized fixed scans ([8df5f69](https://github.com/commandotj/ax-karen/commit/8df5f69d657dfd9d39b6857430c816dbeac06a34))
