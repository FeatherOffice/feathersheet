# Lote de migração de macros — aplicação, mesclagem e fórmulas

**Data:** 12/10/2026. Commits diretos na `main`, com `[skip ci]`.
**Não executei** GitHub Actions, Cargo nem testes compilados neste ambiente.

## 1. `api-aplicacao.ts` e getter de `api-pasta.ts`

Foi verificado previamente que `feathersheet-macros/src/enderecos.rs`
já possuía `rgb_vba` e `vba_cor_hex`. A API Boa agora fornece:

```javascript
const n = app.rgb(255, 0, 128);    // 8388863, BGR do VBA
app.vbaColorToHex(n);             // "#ff0080"
app.batch(() => { sheet.set("A1", 10); sheet.set("B1", 20); });
wb.sheetCount;                    // Usa a lista de abas do Workbook
```

Nenhum conversor de cor foi duplicado; os dois métodos chamam o
interpretador Rust. `batch` é identidade de callback porque o
interpretador Rust já executa **toda a macro** dentro da transação
única e reverte em caso de erro. `sheetCount` reutiliza
`Workbook::nomes`.

Diferença conhecida: a API Rust exige RGB inteiro entre 0 e 255,
enquanto a função TypeScript antiga fazia uma soma aritmética sem
validar os parâmetros. `app.calculate`, `status`,
`setEventsEnabled`, `isError`, `error` e `fn` não foram portados
neste lote. As chamadas que dependem da interface não devem ser
simuladas por armazenamento paralelo.

Os testes estão em `feathersheet-macros/src/testes_aplicacao.rs`.

## 2. Centralização de geometria de mesclagens

A GUI já implementava `app/mesclas.rs` e o núcleo já registrava o
desfazer com `Sheet::alterar_mesclas`. Foi extraída a parte **pura**
de geometria para `feathersheet-core/src/operacoes/mesclagem.rs`:

`cruzam`, `valida`, `normalizar`, `em`, `expandir`, `mesclar`,
`desmesclar`. Esse módulo **não depende de JSON, Iced ou Boa**.

A GUI passou a reutilizar as operações do core e continua
serializando o formato existente `sheet.mc` somente na camada UI.
Também preserva a invalidação da **área expandida inteira** ao mesclar.

O interpretador de macros ganhou suporte a mesclagem em trabalho
concorrente na mesma `main`. Não foi criado outro mecanismo paralelo:
o módulo existente de macro `mesclagem.rs` passou a usar as mesmas
funções de **normalização e cruzamento** do core. Um adaptador
redundante recém-criado, `mesclas.rs`, foi excluído antes de terminar
o lote.

**Diferença intencional de comportamento:** a mesclagem da GUI Rust
mantém conteúdos subordinados ocultos, enquanto a API de macro
legacy `merge` os limpa como no TypeScript. Por isso as duas
camadas compartilham **geometria**, mas não o mesmo método de
aplicação/cancelamento. Não alegar paridade completa entre os dois.

## 3. Eliminar deslocamento duplicado de fórmulas

Já existia `feathersheet-core::operacoes::fill::shift_formula`.
`feathersheet-macros/src/enderecos.rs::deslocar_formula`
tinha outro algoritmo para deslocar A1.

Corrigido o método **canônico** no core para validar linha/coluna
contra `MAX_ROWS` e `MAX_COLS`, usando limites `i64` e emitindo
`#REF!` quando o deslocamento sair da grade. O método da macro
agora é apenas um adaptador para o `shift_formula` existente.
Assim cópia, preenchimento e setFormula passam a usar as mesmas
regras de deslocamento de referências relativas e absolutas.

Testes de borda de planilha foram escritos em
`feathersheet-core/src/operacoes/fill.rs`.

## Política de descarte / validação

`feathersheet/src/macros/api-aplicacao.ts`,
`api-pasta.ts` e `api-navegacao.ts` ainda são importados por
`src/macros/runtime.ts` e contêm funções não migradas.
**Não movê-los isoladamente para `descarte/`.**

A revisão efetuada foi de código no GitHub; não há Cargo/Rustc
disponível neste executor. Portanto estes commits são
**implementados e publicados, mas ainda não compilados ou testados**.

Validação local sugerida, sem GitHub Actions:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-core/Cargo.toml shift_formula -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml testes_aplicacao -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```
