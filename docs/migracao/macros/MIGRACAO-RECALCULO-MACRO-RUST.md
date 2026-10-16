# Migração de `app.calculate()` TypeScript → Rust

Fonte: `feathersheet/src/macros/api-aplicacao.ts`; essa chamada ainda não
existia na API Boa em `feathersheet-macros/src/lib.rs`.

Agora `app.calculate()` usa `__ft("appCalculate", [])` no interpretador
JavaScript e chama `Sheet::invalidar_todos_calculos()` apenas na aba ativa.
Os valores serão recalculados **sob demanda** pelo avaliador Rust existente;
esta é a semântica de execução preguiçosa do FeatherSheet e não dispara uma
varredura para materializar todos os resultados na memória.

A invalidação limpa o cache convencional das fórmulas e marca os resultados
em `FormulaTable` como sujos, preservando padrões e células. A tabela colunar
usa uma passagem no intervalo das linhas ocupadas de cada coluna; evita criar
`Vec<FormulaEntry>` de milhões de elementos. Não cria transação de edição
nem polui Undo e não injeta a operação nas demais abas.

Testes de regressão foram escritos no core, na tabela colunar e no host
de macros, mas **não executados**. Não disparar GitHub Actions.

Para validar localmente:

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-core/Cargo.toml invalidar -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml app_calculate -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```
