# OpenQuery — migração TypeScript → Rust/Iced

Estado em 11/10/2026: **em migração [/]**, ainda **não equivalente** à versão TypeScript.
O TypeScript não deve ser removido antes dos testes de paridade.

## Código já migrado

- Motor Rust: `crates/feathersheet-core/src/consulta/query.rs`, com execução protegida
  `executar_query_limitado` e preflight de cardinalidade em `query/limites.rs`.
- Editor nativo: `crates/feathersheet-gui/src/app/openquery_ui.rs` (menos de 1.000 linhas).
- Assistentes: `openquery_ui/assistentes.rs` (merge de seis modos e chaves compostas,
  acrescentar com alinhamento por nome, pivot com agregação).
- Importador local: `openquery_ui/importador.rs` — CSV, TSV, TXT, JSON e XLSX,
  lê em tarefa separada do Iced, com limites de tamanho e leitura. A atualização
  da origem externa agora oferece reimportação **explícita** do arquivo com
  validação de cabeçalhos, preservando as etapas da consulta.
- Entrada manual: `openquery_ui/manual.rs` usa o parser CSV/TSV existente,
  mantém valores digitados como texto (inclusive `=A1`) e persiste snapshot
  no FSHZ, sem executar fórmulas importadas.
- Executor incremental no core: `consulta/query/fluxo.rs` exporta
  `executar_query_em_fluxo` com callbacks de entrada/saída e contadores
  de `Limite`/`Índice`; executa passos locais sem guardar o conjunto de
  linhas inteiro. **Ainda não integrado à carga de milhões de linhas no Iced**.
  Etapas globais (join, sort, pivot, group, distinct etc.) são recusadas.
- Persistência: `openquery_ui/registro.rs` grava consultas no campo já
  existente `Sheet.oq` do FSHZ, preservando listas TypeScript.
- Atualização: `openquery_ui/execucao.rs` relê fontes internas, substitui
  apenas a área gerada, não duplica a aba e recusa ampliar sobre valores manuais.
- Vínculo de tabela: `openquery_ui/vinculo.rs` cria/atualiza `Sheet.tb`
  com `openQueryId` e `openQueryNome`, com nome estruturado da tabela.
- Etapas: prévia seletiva, reordenação para cima/baixo, exclusão, importação e
  exportação de JSON no contrato de passos do TypeScript.
- UI de consultas: listar, editar, atualizar, desvincular (confirmando sem apagar
  células). Indicação de erro em caso de origem apagada ou destino conflitante.
- Testes unitários: arquivos `openquery_ui/testes.rs`,
  `openquery_ui/{assistentes,importador,execucao,registro,vinculo,json_passos}.rs`
  e `feathersheet-core/src/consulta/query/limites.rs`.

## Falta para equivalência completa (ordem recomendada)

- [ ] **Compilar e corrigir erros** com Cargo, rodar unitários, salvar e reabrir
  FSHZ de verdade, Linux e Windows; ainda não existem evidências de execução
  no ambiente de alteração atual.
- [ ] **Web/API**: migrar extrator seguro de HTML, tabelas e JSON HTTP.
  Suportar importação restrita de URL com proteção contra SSRF, limites
  de download, tamanho e timeout.
- [x] **Entrada manual CSV/TSV** (cabeçalhos na primeira linha, importação
  textual sem interpretação de fórmula). Ainda falta edição tabular célula a
  célula semelhante ao assistente do TS.
- [ ] **Conexões SQL**: definir credenciais seguras e queries somente leitura;
  no TS parte do botão SQL é apenas um placeholder.
- [x] **Destino**: criar nova aba, carregar na aba atual (somente se a
  região de destino estiver totalmente vazia) ou salvar apenas conexão sem
  alterar células. Atualizações respeitam o destino persistido no FSHZ.
  Permanecem pendentes intervalos arbitrários e confirmações avançadas.
- [ ] **Editor avançado**: gerador/visualizador M-like, argumentos por etapa,
  árvore de etapas com operações completas, filtros avançados e preview.
- [ ] **Grandes volumes**: o motor já tem executor de fluxo para operações
  locais, mas ainda falta ligá-lo à grade Iced, gravar resultados grandes em
  lotes com progressos e cancelamento, suportar transformações globais com
  memória delimitada e validação de desempenho. A interface segue limitada
  a 160 mil células por fonte e 240 mil na saída: **não** são 1M+ linhas.
- [x] **Reimportar arquivos externos explicitamente** com seletor de arquivo,
  conferência de cabeçalhos e conservação das etapas. Não há leitura automática
  de caminhos de arquivo persistidos no FSHZ.
- [ ] **Multiplas consultas e dependências**: pastas, ordenação de dependências,
  detecção de ciclo, renomeação/revinculação, atualização em sequência.
- [ ] **Undo**: atualmente a carga tipada invalida o histórico da aba gerada
  para não permitir desfazer apenas parcialmente; implementar transação
  atômica com snapshots delimitados.
- [ ] **QA/UX**: zoom/DPI, telas estreitas, teclado, tema/claro/escuro,
  comparação dos ícones/layout com o TS, arquivos XLSX reais, regressões.

## Comandos no computador de desenvolvimento

```bash
git status
# Somente sem rebase/merge em andamento e sem alterações conflitantes:
git pull --rebase origin main

cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-core/Cargo.toml limites -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml openquery_ui -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Critério para alterar [/] → [x]:** compilação + testes passando,
equivalência funcional dos fluxos realmente usados em TS, atualização e
roundtrip FSHZ/XLSX, performance e inspeção de UI verificados, sem botões
que prometam uma operação não implementada.

### Incremento posterior de 11/10/2026

- Desvincular uma consulta limpa `openQueryId` e `openQueryNome` de
  `Sheet.tb`, preservando tabela e intervalo nomeado; valida a coleção antes
  de escrever. Remover a consulta não deixa vínculo de atualização órfão.
- O preflight de JOIN/APPEND interativo recusa fontes ausentes, modos e chaves
  inválidas antes de executar, em vez de ignorar uma etapa silenciosamente.
- Testes escritos (ainda sem Cargo) abrangem entrada manual, reimportação com
  alteração de cabeçalho, desvinculação e paridade incremental do executor
  Rust com a execução vetorial para operações locais.

### Continuação — destinos e proteções (11/10/2026)

- Seletor **Nova aba / Aba atual / Apenas conexão** no editor Iced.
- Nova consulta em **Aba atual** valida previamente todas as células da área
  de resultado, recusando dados e estilos já existentes. Não destrói a origem;
  a aba e o modo ficam vinculados ao registro da consulta.
- **Apenas conexão** grava somente `Sheet.oq` no FSHZ, sem criar aba,
  tabela ou valores. Consultas de conexão podem ser reabertas e atualizadas
  sem gerar células na grade.
- Testes novos de colisão em aba atual, conexão FSHZ e reabertura, sem
  alegação de aprovação antes da execução Cargo.

## Migração e descarte em 12/10/2026 (sem CI)

- Gerador TypeScript `src/openquery/core/script-generator.ts` **retirado** da árvore ativa, após revisar importações dos módulos OpenQuery e da integração principal. A cópia original idêntica está em `descarte/feathersheet/openquery/script-generator.ts.legacy.txt`.
- Novo `crates/feathersheet-gui/src/app/openquery_ui/script.rs`: tradução nativa do gerador de script; exibido na aba **Script (Rust)**, apenas como referência. A execução permanece no motor Rust, não em JS arbitrário.
- Novo `crates/feathersheet-gui/src/app/openquery_ui/qualidade.rs`: porta a inferência de tipos e diagnóstico de valores válidos, vazios e erros de `ui/components/preview-grid.ts`, amostrando no máximo 500 linhas por coluna exibida.
- O componente Web `src/openquery/ui/components/preview-grid.ts` **não** foi descartado: ainda tem importação ativa por `ui/modal.ts` e é necessário à versão TypeScript.
- Auditoria local (sem rede): `python3 scripts/descarte/verificar_referencias.py <caminho-original.ts>`.
- Sem disparar GitHub Actions/CI; as alterações foram commits `[skip ci]`. Nem compilação nem testes locais foram executados nesta sessão.
