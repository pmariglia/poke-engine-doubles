# Changelog

## [v0.0.7](https://github.com/pmariglia/poke-engine-doubles/releases/tag/v0.0.7) - 2025-08-13

### Features

- Implement commander - ([de0dd0e](https://github.com/pmariglia/poke-engine-doubles/commit/de0dd0ee587001d3ed50cee265608f76587a847c))

- Electroshot in rain doesnt charge - ([1823028](https://github.com/pmariglia/poke-engine-doubles/commit/18230284dbdc7c02b30c425f491cc37bc26431f8))

- (incorrect) direclaw - ([9570f82](https://github.com/pmariglia/poke-engine-doubles/commit/9570f821cc87177bfb9e4e1ea5ea4b91d229a75d))

- Times_attacked flag on Pokemon and implement RageFist - ([c8a5b7c](https://github.com/pmariglia/poke-engine-doubles/commit/c8a5b7c72b6d7f3346543680fd525a857188c42a))

- Calculate damage via io.rs - ([d33b80a](https://github.com/pmariglia/poke-engine-doubles/commit/d33b80ad3822a509c110a6e94904f475c847ab83))

- Stellar boosting all types but only once - ([2380e7a](https://github.com/pmariglia/poke-engine-doubles/commit/2380e7ab6705d11130f9a6bdb42e121e01665e90))


### Bug Fixes

- Fix bug with sleeptalk - ([756285c](https://github.com/pmariglia/poke-engine-doubles/commit/756285cdba661abfb8b8fd8cc54c865c88cf3daa))

- Choice items don't prevent you from selecting a move - ([709cf25](https://github.com/pmariglia/poke-engine-doubles/commit/709cf25e95e248913ef4b3e15863d74a18441c1b))

- Curse has no target - ([0041d14](https://github.com/pmariglia/poke-engine-doubles/commit/0041d14628cae9c7ffb4c78d56880095792ca9cb))

- Direct boosting moves only boost once - ([294ff22](https://github.com/pmariglia/poke-engine-doubles/commit/294ff22302c131f11449960bcd5d4bee08cdf418))

- Getting defender types doesn't short-curcuit when terastallized - ([b5b4971](https://github.com/pmariglia/poke-engine-doubles/commit/b5b4971ec435b39fa56d10058e55508dfd236b4a))

- Calculating damage needs to modify the move before execution - ([746899d](https://github.com/pmariglia/poke-engine-doubles/commit/746899deec23da8d5e5833d932e60dfd08bda171))

- Only boosts targetting user are skipped until final run move - ([bf617d2](https://github.com/pmariglia/poke-engine-doubles/commit/bf617d2f80c2ae14a35496ce6f53d286f6ff131c))

- Has type should not return Stellar - ([ccb3731](https://github.com/pmariglia/poke-engine-doubles/commit/ccb373176459459fdd53c90d576580bf85f9e59a))


### Refactor

- Use usize to index pokemon - ([878be33](https://github.com/pmariglia/poke-engine-doubles/commit/878be3311164dcf2514c28614f87de03cdba2dba))

- Use u8 or u16 over usize for items in Node that don't grow too big - ([1123c6c](https://github.com/pmariglia/poke-engine-doubles/commit/1123c6ceaf3db90d200ae36dd8f5cb1790004d92))

- Lazily Expand Nodes - ([aeeba14](https://github.com/pmariglia/poke-engine-doubles/commit/aeeba144991a259cea29ba045dd6d22bfc91abc8))

- Use option instead of raw pointer for s1/s2 options - ([ab223f2](https://github.com/pmariglia/poke-engine-doubles/commit/ab223f20d169e28d8436a1eebbb0eba58aa637b1))

- PokemonName::None signifies that a pokemon is not part of the battle - ([9f244f7](https://github.com/pmariglia/poke-engine-doubles/commit/9f244f77646f0cbbeba8b45efd6e938ad05175af))

- Boost checking function applies the boost to the state - ([0d8d4c0](https://github.com/pmariglia/poke-engine-doubles/commit/0d8d4c0c20b6be96e4e68546519103caa02169be))

- Dont apply stellar boosted flag if target protects - ([51c847d](https://github.com/pmariglia/poke-engine-doubles/commit/51c847da33a1746093038fd7fe677673f6c0b915))

## [v0.0.6](https://github.com/pmariglia/poke-engine-doubles/releases/tag/v0.0.6) - 2025-07-31

### Refactor

- Evaluation considers other speed modifies when looking at who is faster - ([602d1a9](https://github.com/pmariglia/poke-engine-doubles/commit/602d1a9e0ceb65d2aa2f42e4b2983c4ada3e3de7))

- Python bindings a little less sucky - ([794c77b](https://github.com/pmariglia/poke-engine-doubles/commit/794c77bf7c75b33823b189d7d2471b39ddfa3cd2))

## [v0.0.5](https://github.com/pmariglia/poke-engine-doubles/releases/tag/v0.0.5) - 2025-07-27

### Performance

- Until 50x visits per option are done, choose iteratively - ([8d0f85c](https://github.com/pmariglia/poke-engine-doubles/commit/8d0f85cbe85bd538a773fe92d62a51b127103fbb))

## [v0.0.3](https://github.com/pmariglia/poke-engine-doubles/releases/tag/v0.0.3) - 2025-07-27

### Features
- Python bindings refactor

## [v0.0.0](https://github.com/pmariglia/poke-engine-doubles/releases/tag/v0.0.0) - 2025-04-21

### Features

- Doubles lol