# Migração de INDEX, MATCH, VLOOKUP e HLOOKUP da API de macros

**Data:** 12/10/2026. Commits diretamente na `main`, com
`[skip ci]`. Sem GitHub Actions; compilação e testes ainda
**não executados** neste ambiente.

## Auditoria de duplicações

Origem TypeScript: `feathersheet/src/macros/fn-planilha.ts`,
`fn.index` e `fn.match`.

Foram examinados antes da criação os seguintes módulos Rust:

- `feathersheet-core/src/funcoes/fn_lookup.rs`: já contém
  `fn_index` e `fn_match` do **motor de fórmulas**. Essas funções
  dependem de `Node`, `Evaluator` e `Rect`, enquanto as macros
  recebem **arrays/matrizes JavaScript**. Não foram copiadas para
  `feathersheet-macros`.
- `feathersheet-core/src/celulas/values.rs::compare` foi analisado, mas
  usa ordenação/coerção de tipos diferente da igualdade estrita do
  JavaScript. O adaptador mantém apenas os **testes escalares mínimos**
  de tipo/igualdade e ordenação exigidos por `fn-planilha.ts`; não
  recria o avaliador ou a expressão de fórmula.
- `feathersheet-core/src/funcoes/fn_helpers.rs::make_predicate`:
  interpreta padrões com `*` e `?`, inclusive escapes com `~`.
  A macro usa essa mesma função, sem outra expressão regular.
- `feathersheet-macros/src/funcoes_criterio.rs::achatar`:
  reaproveitado para navegar pelas listas sem clonar matrizes.
  A adaptação de igualdade estrita dos escalares fica restrita a
  `funcoes_busca.rs`; o conversor de critérios permanece privado.

**Foi criado apenas o adaptador**
`feathersheet-macros/src/funcoes_busca.rs` para seleção em
arrays/matrizes das macros e o mapeamento de falhas para `CellError`.
O mesmo adaptador dá acesso a `VLOOKUP` e `HLOOKUP`, sem transpor
matrizes inteiras em memória.
Não existe outra planilha, parser de fórmulas ou avaliador independente.

## API suportada

```javascript
app.fn.index([10, 20, 30], 2);           // 20
app.fn.index([[1, 2], [3, 4]], 2, 1);   // 3
app.fn.index(sheet.range("A1:C10").values(), 2, 3);

app.fn.match("BANANA", ["uva", "banana", "maçã"], 0); // 2
app.fn.match("ab?", ["abc", "abd", "zzz"]);           // 1
app.fn.match(8, [1, 9, 7, 6], 1);                   // 4
app.fn.vlookup('B', [['A',10],['B',20]], 2);           // 20
app.fn.hlookup(5, [[2,5],[20,50]], 2);              // 50
```

O comportamento de `match` mantém o contrato **da API macro TS**,
que difere dos defaults da fórmula de planilha: tipo `0` (exato)
por padrão; `1` retorna o **último** elemento de tipo compatível
menor/igual; `-1` o último maior/igual. Resultados exatos usam
comparação insensível a maiúsculas para textos.

`index` usa índices base 1. Fora do intervalo, retorna
`CellError("#REF!")`. `match` sem correspondência retorna
`CellError("#N/A")`, não uma exceção. O host já tem
`__ftRevive`, que converte marcadores JSON
`{"__ftCellError":"#N/A"}` em objeto JavaScript consultável por
`app.isError`, `.code` e `String(...)`.

Os retornos podem ser escalares de qualquer tipo JSON; a matriz
original não é materializada uma segunda vez. `match` usa o
teto de 500 mil valores e a profundidade máxima 64 do adaptador
existente; buscas não modificam o workbook ou o histórico.

## Diferenças/pendências

- Comparação de strings usa o comparador Unicode do core e pode
  divergir da collation JavaScript/locale em casos especiais.
- Igualdade por identidade de objetos JS não é preservada na
  serialização JSON. O contrato principal suporta números,
  booleanos, strings, nulos e `CellError`.
- `VLOOKUP` e `HLOOKUP` já estão conectados por listas JS.
  `app.error` para gravação de valores de erro nativos segue pendente.
- O legado `fn-planilha.ts` e seus importadores continuam ativos
  no TypeScript. **Não movido para `descarte/`.**

Testes unitários em `funcoes_busca.rs` e integração Boa em
`testes_busca.rs`; ainda não executados. Validação local:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
```
