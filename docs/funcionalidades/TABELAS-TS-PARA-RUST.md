# Tabelas FeatherSheet — migração TypeScript → Rust/Iced

Implementação incremental publicada na `main` em **08/10/2026**.
**Status: entregue para validação, NÃO homologado**. Não foi possível executar
`cargo`/`rustc` neste ambiente. Os testes e o smoke da aplicação precisam
ser executados pelo mantenedor. A branch é compartilhada com outras frentes.

## Paridade rastreável

| Recurso legado TypeScript | Implementação Rust/Iced | Situação |
| --- | --- | --- |
| `ui/app/tabelas.ts::criarTabela` | `app/tabelas.rs::criar`, `App::TabelaAbrir` | Implementado, Cargo pendente |
| `core/table.ts::regiaoAtual`, `pareceCabecalho` | `core/operacoes/ops.rs` usado por `tabelas.rs::sugerir` | Reutilizado |
| `modelo/tabela.ts::nomeDisponivel`, `novoId` | `core/operacoes/tabela.rs::nome_disponivel`, ID `tab_<nome>` | Equivalente para nomes, não mesmo gerador de IDs |
| Nome registrado para fórmulas | `Workbook::definir_nome`/ `remover_nome`; atualizar na expansão | Implementado |
| `mudarEstiloDaTabela` (6 paletas) | `tabelas/migracao.rs::estilo`, `tabelas.rs::paleta` | Implementado |
| `alternarFaixasDaTabela` | `Estilo::Faixas` e Canvas | Implementado |
| Faixas por coluna / borda | `Estilo::FaixasColuna`, `Borda`, Canvas | Implementado |
| `alternarTotaisDaTabela` | `Estilo::Totais` + `Grade` virtual | Implementado; atenção à performance em milhões de linhas |
| `modelo/tabela.ts::totalDaColuna` | `core/operacoes/tabela.rs::total_da_coluna` | Reutilizado |
| Funções soma/média/contagem/máximo/mínimo/desvio | `Estilo::TotalColuna`, cálculo no motor Rust | Implementado |
| `converterTabelaEmIntervalo` | `migracao.rs::converter`, preservação das células | Implementado |
| `renomearTabela` | `migracao.rs::renomear`, atualização do nome definido | Implementado |
| `aplicarFiltroTabela` e `limparFiltroTabela` | `migracao.rs::filtrar`, `core/operacoes/filtro.rs` | Implementado |
| `reavaliarFiltrosTabela` / esconder linhas | `filtro::linhas_escondidas`, alturas e backup em `_featherAlturas` | Implementado, smoke visual pendente |
| `ordenarTabela` | `ops::planejar_ordenacao` + `aplicar_ordenacao` | Implementado |
| `toggleFilter` / Ctrl+Shift+L | `migracao.rs::alternar_auto_filtro`, Ribbon, `App` | Implementado |
| `filter-popup.ts`: filtros por regra | `painel.rs`: contém/começa/termina/igual/comparações numéricas | Implementado com menos operadores que o popup TS |
| `filter-popup.ts`: valores distintos | `painel.rs` pesquisa/multisseleção; `distintos_lote` com varredura progressiva | Implementado para validar em Cargo/GUI |
| `filter-popup.ts`: lista cortada | `Acao::AplicarLista` gera `not_in` dos desmarcados | Implementado |
| `grade/desenho-celulas.ts`: setas filtro | `grade.rs` Canvas + evento de clique por coluna | Implementado |
| `modelo/tabela.ts::crescerTabela` | `migracao.rs::crescer` ao digitar; `crescer_colagem` em bloco | Implementado |
| Redimensionar intervalo da tabela | `migracao.rs::redimensionar`; Ribbon contextual `painel.rs` e `faixa_nativa_grupos.rs` | Implementado em 11/10/2026; Cargo/GUI pendentes |
| `core/table.ts::ajustarRect` após estrutura | `refluir` com `core/operacoes/reflow.rs` | Implementado |
| `ui/barras/ribbon-tabela.ts`: ferramentas visuais | `app/faixa_nativa.rs` + `painel.rs` | Implementado na aba contextual **Ferramentas de Tabela** e no popup de filtro Iced, com aceite visual pendente |
| `tabelasParaJSON`, `tabelasDoJSON` | `Sheet.tb` JSON e codec FSHZ existentes | Compatibilidade estrutural preservada, testes pendentes |
| Undo/redo de visual, totais, criação e metadados sem filtro | `sheet/edicao.rs::alterar_tabelas` + `migracao.rs::mudar_estilo_com_historico` | Implementado parcialmente; Cargo pendente |

## O que ainda NÃO é paridade completa

- O novo popup nativo de filtro é aberto junto ao cabeçalho e a Ribbon oferece aba contextual **Ferramentas de Tabela**; o comportamento exato de foco, pesquisa, ordenação e posição ainda depende de smoke visual.
- **11/10/2026 — novos filtros Rust/Iced:** critérios **Está vazio**, **Não está vazio** e comando **Limpar todos os filtros** no menu do cabeçalho. Preserva alturas zero previamente ocultas manualmente, além das alturas personalizadas; testes de vários critérios, acionamento na UI e round-trip FSHZ foram acrescentados. Aceite por Cargo e smoke visual ainda pendente.
- **11/10/2026 — performance dos filtros Rust:** removida uma busca quadrática entre backups de altura e faixas de linhas ocultas; o filtro restaura alturas antes da nova aplicação e captura o backup em uma única passagem. Comparações de lista e `not_in` agora usam `HashSet` no core, sem percorrer os valores marcados em cada linha. O AutoFiltro limpa todas as colunas filtradas de uma vez, sem varreduras repetidas. Testes de alternância de critérios em 4.096 linhas, dois filtros combinados e motor de 1.000.001 posições sintéticas. **Importante:** a avaliação ainda é linear no número de linhas; altura 0 ainda é gravada por linha oculta no modelo atual.
- O popup nativo agora virtualiza as caixas com viewport de 18 itens e
  usa lotes de 8.192 linhas, com carregar mais/tudo/parar. Ainda faltam
  smoke visual de rolagem, foco, mudança de coluna e alguns operadores do TS.
- A lista não tem mais teto fixo de 100 distintos ou 50 mil linhas:
  a varredura pode continuar até o fim, sob comando do usuário.
  Enquanto incompleta, `not_in` preserva valores ainda não vistos;
  a **avaliação dos critérios** continua feita sobre todas as linhas.
- Totais virtuais ainda são calculados quando a linha aparece no Canvas;
  **é necessário cache por revisão** para uma experiência ótima nas tabelas
  de milhões de linhas.
- **Desfazer/refazer das tabelas:** tema, faixas, bordas e totais usam
  `Sheet::alterar_tabelas` com histórico compacto do JSON `tb`. A criação,
  inclusive dos cabeçalhos automáticos, é agrupada numa transação de Ctrl+Z.
  Renomear, converter em intervalo **sem filtro** e redimensionar **sem filtro**
  também geram histórico. AutoFiltro sem critérios também pode ser ativado e desativado
  com Ctrl+Z/Ctrl+Y, sem copiar os dados das células. Ao desfazer/refazer, a GUI compara metadados antes
  e depois e reconcilia somente os nomes definidos que pertencem às tabelas,
  preservando os nomes independentes. O redimensionamento usa o formulário
  inline A1:C20 na Ribbon contextual. **Pendente:** alterações de filtro e
  conversão/redimensionamento com filtro ativo precisam registrar/restaurar
  alturas de linhas junto com os metadados; não devem gerar undo parcial.
- Histórico e transformações de estruturas complexas (múltiplos filtros e
  ordenações com `Sheet.tb`) exigem round-trip/aceite com fixtures reais.
- Filtro e estilo ainda não são exportados como um objeto de tabela
  editável no XLSX: o FSHZ é o formato primário deste trabalho.

## Estrutura e decisões

- **A fonte dos valores é sempre a engine colunar** `feathersheet-core`:
  `Sheet::data`, `Sheet::formulas` e `Workbook::valor` permanecem iguais.
- **Metadados** da tabela são JSON `TabelaDaAba` em `Sheet.tb`.
  As extensões privadas `_featherOcultas` e `_featherAlturas` auxiliam a
  restauração das alturas sem perder os filtros gravados; o codec FSHZ
  mantém o JSON desconhecido sem tentar reescrevê-lo.
- **Canvas Iced** pinta somente células visíveis. Cores de tabela não criam
  milhões de objetos `Style`. Regras condicionais e estilos explícitos têm
  precedência visual.
- **Nomes definidos** são criados junto com a tabela (e removidos ao converter),
  conforme uso no TypeScript; nomes existentes não são sobrescritos.
- Tabelas usam a grade real de **100.000.000 linhas × 16.384 colunas**.
  Não há limite artificial de 128 colunas na validação do intervalo.
- Ao expandir por colagem, a função `crescer_colagem` avalia metadados uma
  vez por operação e rejeita interseção com outra tabela.

## COMO TESTAR NO MEU COMPUTADOR

Execute na raiz do repositório:

```bash
git status --short
git pull --ff-only origin main
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml tabelas -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

### Teste manual integrado

1. Em A1:C5, crie dados com títulos, números e categorias repetidas. Selecione
   A1 e pressione `Ctrl+T`. Confirme nome/cabeçalho e salve o FSHZ.
2. Vá a **Dados → Design da tabela** dentro do intervalo. Renomeie a tabela,
   alterne paleta e faixas por linha/coluna, ligue borda e totais e escolha
   soma/média/contagem para uma coluna.
3. Clique na seta do cabeçalho; use busca/checkbox, aplique dois critérios,
   confira quais linhas ficam ocultas, remova os filtros e verifique as
   alturas de linha. Confira ordenação A→Z e Z→A.
4. Em outra região, use `Ctrl+Shift+L` para criar AutoFiltro; clique no
   cabeçalho, filtre, remova os critérios e desative o AutoFiltro.
5. Digite logo abaixo e cole várias linhas contíguas. Confira crescimento
   do intervalo e que tabelas vizinhas não são engolidas.
6. Insira e exclua linhas/colunas no meio da tabela; confira nova área.
7. Salve, reabra e confira o nome, a área, o estilo, os critérios e a
   linha de totais; converta em intervalo e confira que os valores persistem.

**Aceite:** mantenha `[/]` enquanto a compilação, os testes Rust, o
round-trip FSHZ e a validação visual não forem aprovados no host Linux.
