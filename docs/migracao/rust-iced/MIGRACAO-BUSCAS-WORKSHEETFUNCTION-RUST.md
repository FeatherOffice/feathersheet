# Migração incremental: funções de busca de macros TS → Rust

Em 12/10/2026, após conferir a API ativa em `feathersheet/crates/feathersheet-macros/src/lib.rs` e confirmar a ausência dos métodos, os algoritmos `vlookup`, `hlookup`, `match` e `index` de `feathersheet/src/macros/fn-planilha.ts` foram expostos pela API JavaScript do Boa, mas executados nativamente em `funcoes_busca.rs`.

- Retorno `CellError` preservado no JavaScript via o marcador existente `__ftCellError`/`__ftRevive`; não transforma `#N/A` e `#REF!` em strings comuns.
- Mantém índices baseados em 1, busca exata por padrão, comparação aproximada e curingas no `match` pelo predicado canônico Rust.
- `hlookup` não precisa criar/transpor matriz; lê diretamente por índice.
- Proteção de até 500 mil valores para acesso a matrizes, e limite de profundidade para varreduras de vetores do host já existente.
- Não foi criado outro interpretador Rust/JS, nem duplicadas funções de fórmula do core.
- Testes unitários e testes de macros incluídos, **não executados** porque o ambiente atual não possui Cargo. Sem GitHub Actions.

Validação local: `cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2`.
