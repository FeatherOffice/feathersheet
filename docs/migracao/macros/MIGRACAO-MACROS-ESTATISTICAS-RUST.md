# Migração estatística das macros: average, median e averageIf

**Data:** 12/10/2026. Branch `main`, commits `[skip ci]`,
sem GitHub Actions; testes escritos, não executados nesta sessão.

## Auditoria antes da implementação

Fonte original: `feathersheet/src/macros/fn-planilha.ts`
e `src/macros/api-aplicacao.ts`.

O Rust já possuía as implementações para **fórmulas**
`fn_stats::fn_average`, `fn_stats::fn_median` e
`fn_stats::fn_averageif`. A chamada por macro, porém, recebe
arrays JSON/JavaScript, e não nós `Node` do parser de fórmulas.

Em vez de reimplementar média e mediana no crate de macros, a parte
numérica pura foi exposta pelo próprio core:

- `feathersheet-core::funcoes::fn_stats::media_numerica`
- `feathersheet-core::funcoes::fn_stats::mediana_numerica`

Os antigos `fn_average` e `fn_median` também usam essas funções.
`averageIf` das macros filtra pelos **mesmos critérios** que
`countIf`/`sumIf`, mediante o `make_predicate` já existente
em `fn_helpers`, e calcula a média com `media_numerica`.
Nenhuma segunda implementação de comparação com curingas foi criada.

## API JavaScript disponível no Boa

```javascript
app.fn.average([2, 4, 6]);                          // 4
app.fn.median([10, 2, 7, 5]);                       // 6
app.fn.averageIf([2, 6, 9], ">5");                  // 7.5
app.fn.averageIf([2, 6, 9], ">5", [100,200,300]); // 250

const resultado = app.fn.averageIf([2, 4], ">100");
app.isError(resultado); // true
resultado.code;         // "#DIV/0!"
```

## Erros tipados

Os códigos de erro são os de
`feathersheet-core::celulas::values::ErrorCode`.
Uma resposta de erro sai do Rust com marcador JSON reservado
`{"__ftCellError":"#DIV/0!"}`, que a camada JS do interpretador
converte em uma instância `__CellError` com `code`,
`detail` e `toString()`.

Também há conversão de volta para o tipo JavaScript ao ler
`sheet.get` ou `sheet.range(...).values()`. Assim, um
**erro calculado** não é confundido com a string literal
`"#DIV/0!"`: `app.isError` retorna `true` apenas para
instâncias do objeto de erro.

Códigos esperados:
- Média sem número: `#DIV/0!`.
- Mediana sem número: `#NUM!`.
- Média condicional sem número selecionado: `#DIV/0!`.

**Limitação intencional:** esse lote cobre **retorno/leitura**
de `CellError` nas macros. Gravar `app.error(...)` em uma célula
como um valor de erro nativo requer um caminho de escrita que
não degrade o valor para texto. Não declaramos `app.error`
concluído, nem alteramos a semântica de `sheet.set` neste lote.
`vlookup`, `hlookup`, `match` e `index` também continuam
pendentes.

## Descarte e testes

`fn-planilha.ts` continua importado pelo runtime TS e tem
outras funções não migradas. **Não arquivar nem apagar agora.**

Testes adicionados em
`feathersheet-macros/src/funcoes_criterio.rs` e
`feathersheet-macros/src/testes_aplicacao.rs`, cobrindo
agregações numéricas, médias condicionais, os erros `#DIV/0!`
e `#NUM!`, `app.isError`, texto literal versus erro calculado
e rollback quando um critério inválido causa falha.

**Verificação:** alterações confirmadas por leitura dos arquivos
no GitHub. Compilação e testes ainda não foram executados. Para
conferir localmente, sem consumir créditos de CI:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
```
