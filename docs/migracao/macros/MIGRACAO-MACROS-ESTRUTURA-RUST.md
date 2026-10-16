# Migração da estrutura de linhas e colunas nas macros

**Data:** 12/10/2026. Repositório `M4R4G0N/FeatherOffice`, branch
`main`, commits `[skip ci]`. Nenhum GitHub Actions acionado.

## Auditoria antes de implementar

Origem: `feathersheet/src/macros/api-estrutura.ts`.

No core Rust já existem:

- `Sheet::definir_dimensao(coluna,indice,px)`, com atualização do
  histórico de dimensões;
- `Sheet::alterar_visibilidade_dimensoes(coluna,ocultar,inicio,fim)`,
  incluindo memória de largura/altura anterior e desfazer;
- `enderecos::coluna_texto`, `coluna_numero`, `linha_numero`,
  para o contrato VBA (base 1).

**Não** foram duplicados esses motores. A novidade é somente
`feathersheet-macros/src/estrutura.rs`, um adaptador JSON/JS
que valida argumentos e chama essas APIs do core.

## Comandos expostos ao interpretador Boa

| JavaScript | Operação existente reutilizada |
| --- | --- |
| `sheet.setRowHeight(2,45)` | `definir_dimensao(false,1,45)` |
| `sheet.setColumnWidth("C",144)` | `definir_dimensao(true,2,144)` |
| `sheet.hideRow(3)` | `alterar_visibilidade_dimensoes(false,true,2,2)` |
| `sheet.hideColumn("AA")` | `alterar_visibilidade_dimensoes(true,true,26,26)` |
| `sheet.hideRow(3,false)` | `alterar_visibilidade_dimensoes(false,false,2,2)` |
| `sheet.hideColumn("C",false)` | `alterar_visibilidade_dimensoes(true,false,2,2)` |
| `sheet.rowCount("A1:A100")` | Calcula altura de `Rect`, sem acessar as células |
| `sheet.columnCount("C:C")` | Calcula largura de `Rect`, sem varrer as 10M linhas |

O índice de linha e coluna recebido pela API JS começa em **1**;
colunas também aceitam letras de A a XFD. Operações de dimensões
participam da transação global da macro, inclusive em
`wb.sheet("Outra").setRowHeight(...)` e em reversão por erro.
Operações idênticas (nenhuma alteração) não são marcadas como mudança.

`setRowHeight(2,0)` e `setColumnWidth("B",0)` escondem,
usando o motor nativo de visibilidade. Para tamanho positivo, o core
armazena pixels inteiros, então o adaptador arredonda frações e
valida `0..=10000`, em vez de criar persistência paralela de floats.

`rowCount` e `columnCount` são leituras de metadados de intervalo:
também aceitam seleções grandes como `C:C` e `1:1`. Não materializam
células, texto ou histórico.

## Pendências e diferenças conscientes

- O **core** ao reexibir uma linha/coluna estende a operação às
  dimensões ocultas contíguas (comportamento da interface), enquanto
  o TS `hideRow(...,false)` removia somente o índice indicado. A API
  de macros Rust herda a regra do core; ainda não há paridade estrita
  de reexibição isolada.
- `autoFitColumn` depende do cálculo de largura na interface Iced;
  ainda não foi integrado a este host.
- `insertRow`, `deleteRow`, `insertColumn` e `deleteColumn`
  requerem coordenação do estado/mesclagens/regras e undo da pasta;
  já existem operações estruturais no `Workbook`, mas não foram
  chamadas via macro nesta etapa para evitar uma integração parcial.
- A versão antiga `src/macros/api-estrutura.ts` **continua importada
  pelo runtime TypeScript**. Não mover para `descarte/` antes de
  substituir os consumidores ativos.
- `Sheet::definir_dimensao` persiste largura/altura somente como u32;
  valores de fração de pixel não mantêm a precisão de TS.

## Verificação

Testes de regressão **escritos** em
`feathersheet-macros/src/estrutura.rs` e
`feathersheet-macros/src/testes_estrutura.rs`. Cobrem desfazer de
duas dimensões com uma macro, ocultação, validação de limites,
reversão se o script falhar, outras abas e contagem de intervalos de
100 milhões de linhas.

**Não executados neste ambiente.** Verificação local recomendada:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml estrutura -j 2
```

Os testes via GitHub Actions não foram executados nem solicitados.
