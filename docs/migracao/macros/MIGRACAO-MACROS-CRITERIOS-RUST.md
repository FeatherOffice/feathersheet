# Migração da API de macros: COUNTIF, SUMIF e identificação de abas

**Data:** 12/10/2026. Implementação enviada para a `main` com
mensagens `[skip ci]`, sem executar GitHub Actions ou Cargo.

## Auditoria de duplicações

Fontes TypeScript:

- `feathersheet/src/macros/fn-planilha.ts` — `fn.countIf`,
  `fn.sumIf` e interpretação de critérios do Excel.
- `feathersheet/src/macros/api-aplicacao.ts` — `app.fn`.
- `feathersheet/src/macros/api-pasta.ts` — localização de abas
  por nome sem distinguir maiúsculas.

O core Rust **já contém**:

- `feathersheet-core/src/funcoes/fn_helpers.rs::make_predicate`,
  inclusive operadores `>`, `<`, `<>`, `=`, `>=`, `<=` e curingas
  `*` e `?`. As macros passam o critério para essa mesma função;
  **não existe uma segunda implementação de comparação**.
- `Workbook::indice_por_nome` faz busca de nomes de abas
  case-insensitive. A API JavaScript usa essa operação por um despacho
  de leitura (`indiceAba`), sem duplicar a busca em JavaScript.

## Operações integradas no interpretador Boa

```javascript
app.fn.countIf([[2,6],[9,null],[20]], ">5");      // 3
app.fn.sumIf([2,6,9], ">5");                        // 15
app.fn.sumIf([2,6,9], ">5", [100,200,300]);       // 500

// Contrato antigo de api-pasta.ts: busca ignorando caixa e espaços.
wb.sheet("  dados de energia  ").set("A1", 42);
wb.sheet(2).get("A1");
```

`countIf` e `sumIf` recebem valores de arrays/matrizes JavaScript,
inclusive os resultados de `sheet.range(...).values()`. O adaptador
`feathersheet-macros/src/funcoes_criterio.rs` achata arrays por
**referências emprestadas**, não duplica os elementos, aplica o
predicado do core e retorna número. Quando o critério é numérico (`>5`, `=3`, `5`), tipos não numéricos são ignorados, como no TS. A exceção é `<>5`, que seleciona também valores não numéricos por ser uma negação da igualdade. O predicado de comparação e os curingas continuam no core.

Limites defensivos: máximo de 500 mil valores e profundidade de
aninhamento até 64. `sumIf` ignora valores não numéricos no intervalo
de soma e aceita o intervalo opcional como na API legada. Uma operação
inválida produz erro no interpretador e reverte as demais alterações
feitas na macro. A busca de abas não altera a planilha.

## Diferenças e pendências

- A função `make_predicate` é o **critério canônico das fórmulas Rust**,
  cuja coerção de vazios/texto pode divergir em alguns casos do
  `criterio` TypeScript; comparar dados reais antes de afirmar paridade
  integral em valores especiais.
- Não foram portadas todas as funções de `app.fn`. `averageIf`,
  `average`, `median`, `vlookup`, `hlookup`, `match` e `index`
  exigem decidir também como transportar objetos `CellError` entre
  Boa e o core. Não devolver string como se fosse erro estruturado.
- Não foram migrados `wb.addSheet`, `deleteSheet`, `renameSheet`,
  `copySheet` e `moveSheet`, pois o host deve garantir rollback
  estrutural caso uma macro falhe no meio.
- TypeScript ainda é usado pelo runtime legado. Nada foi movido para
  `descarte/` nesta etapa, evitando imports quebrados.

## Arquivos e validação

- Rust: `feathersheet-macros/src/funcoes_criterio.rs`
- Boa e lookup de abas: `feathersheet-macros/src/lib.rs`
- Testes: `feathersheet-macros/src/testes_aplicacao.rs`

Testes unitários e de integração foram **escritos, não executados**.
Para executar localmente sem GitHub Actions:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
```
