# Migração de `src/macros/api-valores.ts` para Rust

**Data:** 12/10/2026 · commits `[skip ci]` · sem GitHub Actions.

## Funcionalidades novas do interpretador Rust

O motor `feathersheet-macros` agora expõe os comandos
`clearContents`, `clearFormats`, `replace` e `find`
como métodos de `sheet` e `sheet.range(...)`.

| Comando de macro | Implementação Rust |
| --- | --- |
| `sheet.clearContents("A1:B5")` | Apaga valores e fórmulas sem retirar estilos diretos |
| `sheet.clearFormats("A1:B5")` | Retira estilos diretos sem apagar valores/fórmulas nem estilos herdados |
| `sheet.replace("A1:B10", "abril", "maio", {matchCase:false})` | Substitui todas as ocorrências literais no texto digitado e retorna número de células modificadas |
| `sheet.find("vitória", {range:"A1:E100"})` | Retorna endereço A1 da primeira ocorrência ou `null` |
| `sheet.find("=A1", {inFormulas:true})` | Busca na fórmula original em vez de no valor exibido |
| `sheet.range("B1:B20").find("ok", {wholeCell:true})` | Busca limitada à região do objeto |

As ações de edição usam `Sheet::snapshot` e `Sheet::apply`, em
`operacoes_valores.rs`. Isso mantém os estilos diretos do usuário e
participa da transação única do interpretador Boa: Ctrl+Z reverte as
alterações de uma macro, incluindo as alterações feitas por esses
comandos. Sem criar uma nova matriz com todas as células da área.

`find` usa a interseção do intervalo requisitado com a área usada,
percorre linhas e colunas em ordem e limita a busca a 500 mil células.
Em números, a comparação padrão usa o formatador nativo de células.
A busca não modifica a planilha.

## Migração adicional: `copy` com quatro modos

O módulo novo `feathersheet-macros/src/copiar.rs` implementa
`sheet.copy(origem,destino,{paste})` e
`sheet.range(origem).copy(destino,{paste})`:

| `paste` | Conteúdo copiado | Formatação |
| --- | --- | --- |
| `all` (padrão) | Texto original, deslocando fórmulas relativas | Copia o estilo da origem |
| `values` | Valor calculado, sem copiar fórmula | Preserva a formatação do destino |
| `formulas` | Texto digitado e fórmulas com deslocamento A1 | Preserva a formatação do destino |
| `formats` | Não muda conteúdo/fórmula | Copia o estilo da origem |

A origem é fotografada em um plano **antes de alterar o destino**. Desse
modo a cópia sobreposta não passa a ler o próprio resultado; o plano só
retém células com dados/estilos, deixando vazios implícitos. A gravação
usa `Sheet::apply(...,record=true)` na transação da macro e permite Ctrl+Z.

O plano valida a área de origem (máximo 500 mil células) e o tamanho da
área de destino antes de escrever, incluindo bordas de 100 milhões de
linhas e 16.384 colunas. O último retângulo usado é interceptado seguindo
a função `Contexto.usada` do TypeScript, sem expandir automaticamente
para a planilha toda.

A API nativa JavaScript é executada no Boa, mas a cópia é feita pelo
motor **Rust**, sem passar por objetos de células TypeScript. Foram
adicionados testes **não executados** para sobreposição, os quatro
modos, ajuste de fórmulas, estilo, limites e undo.

### Limitações de `copy`

- Ainda funciona somente **dentro da mesma aba**, como
  `apiValores(c).copy()`; referência qualificada de outra aba exige
  comando explícito adequado, ainda pendente.
- O plano materializa somente as **células ocupadas** do intervalo,
  mas grandes intervalos densos podem consumir dezenas de MiB;
  execução totalmente streaming com sobreposição exige outro desenho.
- A conversão da colagem `values` segue o texto digitável, o que pode
  reinterpretar tipos especiais do valor calculado ao colar; paridade
  com detalhes de formatação de datas ainda requer verificação.

### Diferenças conhecidas e limites

- `clearContents`, `clearFormats` e `replace` ainda recusam
  intervalos de mais de 500 mil células, mesmo ao selecionar linha ou
  coluna inteira; o TypeScript antigo podia reduzir essa área à região
  usada. Esse refinamento ainda está pendente.
- `replace` distingue/ignora maiúsculas; o case-folding Unicode
  expansivo (ex.: `ß` e `İ`) não tem paridade completa com regex `gi`
  do JavaScript. Não utiliza regex do usuário.
- `find` é síncrono e limitado a 500 mil células para não congelar o
  aplicativo em pesquisas de milhões de posições.
- As operações de `copy` e `move` já estão conectadas ao Rust;
  `autoFill` foi integrado ao Rust nesta rodada. As pendências são bordas de paridade da colagem
  e do tipo de valores. `getText` e `getAddress` já foram portados.
- Testes unitários foram **escritos**, mas não executados aqui:
  a sessão não disponibiliza Cargo/Rustc. Não houve execução de Actions.

## Descarte do TypeScript

**Não movido:** `feathersheet/src/macros/api-valores.ts`.

O arquivo continua importado por `feathersheet/src/macros/runtime.ts`,
que importa `apiValores` e agrega a API com os demais `api-*.ts`.
Retirar apenas esse arquivo causaria importação quebrada no runtime
legado. A implementação Rust acima é nativa e não depende do TS.

Para remover o TypeScript com segurança, migrar os comandos restantes e
o runtime antigo inteiro (incluindo consumidores e testes) antes de
arquivar o bloco na pasta `descarte/`.

### Fontes

- TS: `feathersheet/src/macros/api-valores.ts` e `macros/runtime.ts`.
- Rust: `feathersheet/crates/feathersheet-macros/src/operacoes_valores.rs`
  e `src/lib.rs` do mesmo crate.

## Migração de `move` (12/10/2026)

A API de macro agora implementa `sheet.move(origem, destino)` e
`sheet.range(origem).move(destino)`. A lógica das células **não foi
duplicada**: o adaptador `feathersheet-macros/src/movimento.rs`
aproveita `Workbook::mover_intervalo`, em
`feathersheet-core/src/planilha/workbook.rs`.

Este motor já transfere conteúdo, estilos e reescreve fórmulas de
referência tanto na aba ativa quanto em outras abas. Operações
sobrepostas também são atendidas. O adaptador valida antes de
modificar: **500 mil células por movimentação**, destino na grade,
endereço dentro da mesma aba e deslocamento no intervalo `i32`.

A compatibilidade com a transação de macro exigiu tratamento
adicional: como `Workbook::mover_intervalo` fecha as próprias
transações das abas afetadas, o adaptador mantém **uma transação
externa** nas abas potencialmente referenciadoras. Ao terminar a
movimentação, conserva abertas apenas aquelas que tiveram versão
alterada e fecha sem efeito as restantes. O host confirma as
transações quando a macro termina ou as reverte quando ela falha;
isso inclui fórmulas corrigidas em outras abas.

Casos de regressão escritos, **não executados**, incluem: movimento
normal, movimentação sobreposta/limites (também já cobertos pelo
motor de `Workbook`), desfazer, erro JavaScript após o movimento
com referência em outra aba e erro após outras escritas.

`feathersheet/src/macros/api-valores.ts` permanece na árvore ativa,
pois `macros/runtime.ts` ainda importa `apiValores`.
Não mover arquivo TS ativo para `descarte/` enquanto esse import
continuar presente.


## Portabilidade adicional de `getText` e `getAddress`

Duas operações de leitura de `src/macros/api-valores.ts` passaram para
o host nativo Rust do interpretador Boa; nenhuma célula é calculada pelo
frontend legado.

- `sheet.getText("B1")`, `sheet.range("B1").getText()` e
  `sheet.range("B1").displayText` leem o **texto exibido**,
  respeitando fórmulas calculadas e formatação numérica.
  `range.text` continua lendo/escrevendo texto bruto, como no TypeScript.
- `sheet.getAddress("D8:B2")` retorna `B2:D8` e aceita
  `MacroRange`/objetos `{address}`, objetos retangulares
  `{r0,c0,r1,c1}` (coordenadas 0-based), e
  `sheet.range("A1:B2").getAddress()`.
- Intervalos como `C:C` não criam matrizes de dez milhões de
  elementos: `getText` examina apenas a primeira célula, enquanto
  `getAddress` e `area` só resolvem limites.
  `getAddress("C:C")` devolve `C1:C100000000`, conforme o getter
  `MacroRange.address` da versão TypeScript.
- Em `operacoes_valores.rs`, a função nativa
  `texto_exibido` é compartilhada com `find`, eliminando a
  duplicação de código do formatador.
- Ambas são **somente leitura**, não alteram célula, histórico
  ou estado de arquivo. Os limites de até 500 mil células continuam
  aplicados às operações que efetivamente percorrem ou escrevem dados.

Referência histórica dos trechos TS em
`descarte/feathersheet/typescript/macros/api-valores-leitura.ts.legacy.txt`.
O arquivo completo `src/macros/api-valores.ts` **permanece na origem**
porque o runtime TS histórico ainda o importa. Não retirar prematuramente.

**Validação:** testes Rust adicionados, não executados.
Sem GitHub Actions/CI. Comandos para execução local:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml get_text -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml get_address -j 2
```

## Porta do `autoFill` das macros (12/10/2026)

A antiga chamada TypeScript `apiValores(c).autoFill(origem, destino)`
agora tem implementação nativa no interpretador Rust Boa:

```javascript
const preenchidas = sheet.autoFill("A1:A2", "A1:A10");
sheet.range("B1").autoFill("B1:B20");
```

O arquivo `feathersheet-macros/src/preenchimento.rs` acrescenta
a validação explícita de origem/destino. A execução utiliza
**`Sheet::preencher` em `feathersheet-core/src/operacoes/fill.rs`**,
que já contém séries numéricas, textos com numeração, expansão de
fórmulas e cópia de estilos. Não há duplicação do algoritmo do TS.

- A fonte deve estar totalmente contida no destino.
- A expansão ocorre em **uma direção por vez** (cima, baixo,
  esquerda ou direita). Extensão simultânea horizontal e vertical
  devolve erro em vez de modificar a grade de modo ambíguo.
- A origem e o destino possuem limite de **500 mil células**;
  o destino precisa estar dentro da planilha.
- Intervalos idênticos retornam `0` sem marcar alterações.
- As novas células participam da transação única da macro e
  são revertidas em um Ctrl+Z ou se o JavaScript lançar exceção.
- O retorno é a quantidade de células **planejadas para preencher**,
  de acordo com `Sheet::preencher` já existente no core.

Foram escritos casos de regressão para série numérica, fórmula,
limites, retorno `0`, desfazer e falha após preenchimento.
**Cargo e GitHub Actions não foram executados**.

`src/macros/api-valores.ts` permanece na origem pois ainda é
importado por `src/macros/runtime.ts`. Arquivá-lo isoladamente
quebraria o runtime legado, portanto não foi removido nesta etapa.
