# APIs de navegação de macros: port incremental para Rust

Origem: `feathersheet/src/macros/api-navegacao.ts` e getters de
`feathersheet/src/macros/runtime.ts`. Os trechos originais foram copiados
para `descarte/feathersheet/typescript/macros/navegacao-leitura.ts.legacy.txt`
como referência não importada; **arquivos TS completos continuam ativos**
enquanto houver consumidores legados.

Comandos agora resolvidos pelo host Rust `feathersheet-macros` (Boa):

| Chamada | Comportamento |
| --- | --- |
| `sheet.getLastRow('C')` ou `sheet.getLastRow(3)` | Última linha com conteúdo naquela coluna; 0 quando não há conteúdo |
| `sheet.getLastColumn(10)` | Última coluna com conteúdo na linha 10; 0 se vazia |
| `sheet.getDataRange()` | MacroRange de A1 até a última célula usada; null em aba vazia |
| `sheet.lastRow`, `sheet.lastColumn` | Limites usados 1-based da aba, como no TS |
| `sheet.address(2,3)` | Endereço C2; valida limites |

Implementação `feathersheet-core/src/planilha/sheet/navegacao_macros.rs`:
os valores são consultados no armazenamento esparso, incluindo fórmulas.
`DataTable::ultima_linha_coluna` e `FormulaTable::ultima_linha_coluna`
consultam os limites internos, sem percorrer dez milhões de linhas virtuais
nem criar lista de células. Estilos isolados não contam como dados.
Leituras não alteram planilha ou histórico de desfazer.

## Complemento: specialCells, intersect e union

Migrados nesta rodada para a API de macros Rust/Boa:

| API JavaScript | Implementação Rust reaproveitada |
| --- | --- |
| `sheet.specialCells("A1:C5","formulas")` | Enumeração ordenada `Sheet::chaves_no_retangulo` e `get_raw` |
| `sheet.range("A1:C5").specialCells("blanks")` | Leitura de vazios dentro do retângulo, sem alterar valores |
| `sheet.specialCells("A1:C5","constants")` | Texto bruto não vazio que não é fórmula |
| `sheet.specialCells("A1:C5","errors")` | Erros calculados por `Workbook::valor`, inclusive fórmulas |
| `sheet.intersect("A1:C5","B2:D6")` | `enderecos::intersecao` já portada do TS; retorna MacroRange ou null |
| `sheet.union("A1","C3")` | `enderecos::uniao` já portada; retorna MacroRange |

**Verificação prévia de duplicação:** `chaves_no_retangulo`,
`get_raw`, `Workbook::valor`, `enderecos::intersecao` e
`enderecos::uniao` já existiam. A nova função
`feathersheet-macros/src/navegacao.rs::celulas_especiais` é somente
um adaptador de seleção e formatação dos endereços A1; não implementa
outro armazenamento, avaliador ou parser de intervalos.

A busca `specialCells` possui teto de **500 mil células**, igual ao
host, e ordena os endereços por linha e coluna. `intersect` e `union`
são metadados de intervalo e não possuem esse teto, pois não
materializam dados nem percorrem células.

**Manutenibilidade:** os testes de integração, antes embutidos no
`feathersheet-macros/src/lib.rs` (1.132 linhas), foram transferidos
sem reescrever a lógica para `src/testes.rs`; o `lib.rs` ficou com
aproximadamente 596 linhas.

**Paridade parcial:** `offset`, `resize`, `columnOf`,
`rowOf` e os comandos de mesclagem por macros ainda
precisam de integração. `getRegion` foi conectado ao core Rust. `api-navegacao.ts` continua importado pelo
runtime TypeScript histórico e **não foi movido para descarte**.

Os novos testes foram escritos, **não executados**. Nenhum GitHub
Actions foi acionado.



Testes unitários Rust escritos para linhas esparsas, fórmulas, aba vazia,
endereço e rollback, **não executados neste ambiente**. CI/Actions não
foram acionados.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml navegacao_macro -j 2
```

## Operações geométricas `offset`, `resize`, `columnOf`, `rowOf`

Essas quatro funções do TypeScript foram integradas ao motor Rust/Boa,
sem duplicar o suporte que já existia para `intersect`, `union` e
`specialCells` no módulo `navegacao.rs`. A implementação reside em
`feathersheet-macros/src/navegacao_intervalos.rs`.

- `sheet.offset("B2:C3", 2, -1)` → `A4:B5`
- `sheet.resize("C7:D9", 2, 3)` → `C7:E8`
- `sheet.columnOf("B2:D5", 2)` → `C2:C5`
- `sheet.rowOf("B2:D5", 3)` → `B4:D4`

Todas retornam `MacroRange`, usam índices internos 0-based e
parâmetros de posição 1-based, validam os limites da planilha e não
varrem nem modificam células, inclusive para referências como `C:C`.
`getRegion` foi conectado ao core Rust. `merge`, `unmerge` e `isMerged` estão implementados na seção de mesclagem abaixo.

Trecho TS original preservado em
`descarte/feathersheet/typescript/macros/navegacao-intervalos.ts.legacy.txt`,
sem remover `src/macros/api-navegacao.ts` que tem outras funções ainda
usadas. Testes Rust escritos, não executados; nenhum GitHub Actions.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml geometria -j 2
```

## `getRegion`: API de macros ligada ao motor Rust

A API antiga em `feathersheet/src/macros/api-navegacao.ts` expunha
`sheet.getRegion("B2")`. A lógica de região conectada com vizinhança
diagonal **já existia** em `feathersheet-core/src/operacoes/ops.rs`
(`regiao_atual`) e já era utilizada por outras telas. Esta rodada só
conecta o host Rust de macros à mesma função, por `__ft('getRegion',...)`,
e devolve um `MacroRange` no interpretador Boa.

- Aceita célula, área com várias colunas ou seleção de coluna inteira
  (`A:A`); a operação de leitura não passa pelo teto de 500 mil
  células usado em operações de varredura/escrita.
- A expansão fica limitada aos limites usados no motor, com diagonal;
  linhas/colunas vazias interrompem a expansão quando não fazem parte
  do retângulo inicial.
- Em aba vazia, mantém o intervalo solicitado.
- Não modifica valores, fórmulas, histórico ou versão do Workbook.
- Adicionados testes de integração para vizinhança diagonal, fórmula,
  bloco afastado, aba nomeada, área inteira, aba vazia, entrada inválida
  e rollback em erro.

**Nota:** esta integração corresponde apenas à API de macros, não a
uma nova implementação do algoritmo nem a uma migração total de
`api-navegacao.ts`. Os métodos `merge`, `unmerge` e `isMerged` já foram integrados posteriormente ao Rust; consultar a seção abaixo.
O trecho TS original foi preservado no descarte temporário.
Testes escritos, **não executados**; nenhum GitHub Actions foi disparado.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml macro_get_region -j 2
```

## Mesclagem pela API de macros: `merge`, `unmerge`, `isMerged`

Estas três chamadas de `src/macros/api-navegacao.ts` passam agora pelo
host Rust do interpretador Boa (`feathersheet-macros/src/mesclagem.rs`).
O núcleo **já** oferece `Rect`, `Sheet::chaves_no_retangulo`,
`Sheet::alterar_mesclas` e transações para desfazer/refazer:
não foi criado outro armazenamento nem outro histórico.

- `sheet.merge("A1:C2")` mescla o retângulo e apaga o conteúdo das
  células subordinadas, preservando o valor/fórmula da âncora.
- `sheet.unmerge("B1")` remove toda mescla tocada pelo intervalo,
  como o TypeScript. Não recria os conteúdos apagados.
- `sheet.isMerged("B1")` pergunta se a **primeira** célula do
  intervalo pertence a alguma mescla. Não grava nada.
- `merge` e `unmerge` devolvem o objeto `sheet` para encadeamento
  e funcionam com `wb.sheet("Dados")` em abas diferentes.
- `merge` limita a 500.000 posições, como `api-contexto.ts`;
  `unmerge` e `isMerged` consultam somente metadados, inclusive para
  seleções inteiras, sem alocar grade virtual.
- Limpar células, atualizar `sheet.mc` e outras escritas da macro
  compartilham uma única transação nativa. Erros posteriores revertem
  células **e** metadados; operações sem efeito não criam undo.

**Diferença deliberada:** o botão visual de mesclagem do Iced
(`app/mesclas.rs`) **preserva** conteúdo subordinado para reaparecer
ao desmesclar; a macro TypeScript original `merge` o **apagava**.
Esta implementação preserva a semântica antiga para não modificar
a saída de macros existentes. O botão gráfico não foi alterado.

Referências TypeScript históricas preservadas em
`descarte/feathersheet/typescript/macros/navegacao-mesclagem.ts.legacy.txt`.
O arquivo `src/macros/api-navegacao.ts` permanece na origem porque
contém outras funções ativas. Foram adicionados testes Rust de undo,
rollback, fórmulas, validação, interseção, abas e mesclas redundantes;
não executados nesta sessão. Nenhum GitHub Actions executado.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml mesclagem -j 2
```
