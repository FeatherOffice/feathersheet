# FeatherSheet — AUDITORIA DO FRONTEND TYPESCRIPT → RUST/ICED (3 APPs × exclusivo)

> **ATUALIZAÇÃO 12/10/2026:** os caminhos `src/ui/`, `src/ajuda/`, `src/editor-macros/` e outras telas TS descritos neste documento são históricos; a UI foi movida para `../../descarte/feathersheet/legacy-frontend/src/`. A nova fonte oficial é Rust/Iced, com web futura por WASM. O quadro de 08/10 continua como inventário de lacunas, não como declaração de suporte ao frontend legado.

> **Leitura do repositório em 08/10/2026 (branch `main`); documento de planejamento, não aceite funcional.** Origem do frontend: [`feathersheet/src/`](../../src/). Destino: [GUI Rust/Iced](../../crates/feathersheet-gui/src/) e [`feather-ui`](../../../feather-ui/src/). Checklist central: [PLANO-INTEGRACAO-CHECKLIST.md](../../../featheroffice/PLANO-INTEGRACAO-CHECKLIST.md). Plano anterior, **não duplicar:** [PLANO-MIGRACAO-ICED.md](PLANO-MIGRACAO-ICED.md). Este arquivo faz o *delta técnico* com o código atual, não replica percentuais de tarefas antigas.

## Incremento — 12/10/2026 — Grade: navegação horizontal por colunas visíveis (TS → Rust)

**Conferência prévia:** `axis.ts` no legado já inspirou o índice esparso
`grade/eixos.rs`; `grade/congelado.rs` já pulava colunas de largura zero
na **renderização**, mas o despacho Rust avançava `coluna_inicio` por
**índice**, desperdiçando giros sobre colunas ocultas. Não foi criado outro
índice ou engine.

- Removidas definições duplicadas de `primeira_linha_visivel` e
  `origem_apos_rolar` em `grade/auxiliares.rs`, que causariam erro de
  símbolo duplicado em `impl Grade`.
- `Eixo::deslocar_visiveis` avança ou retrocede por colunas visíveis,
  pulando intervalos ocultos via `proximo_visivel` / `anterior_visivel`.
  Mantém o limite de colunas congeladas.
- `EstadoEixos::coluna_apos_rolar` usa o cache por ID/versão de aba,
  sem copiar dados de células ou reconstruir o eixo por evento.
- `Msg::Rolar` e `Msg::RolarNaGrade` chamam o método único
  `App::rolar_colunas_visiveis` da tela nativa. O eixo horizontal
  passa a contar colunas desenháveis e não índices ocultos.
- Após **ocultar** linhas ou colunas, a seleção ativa agora procura
  o próximo índice desenhável usando `EstadoEixos::visivel_apos_ocultar`.
  Removidas duas varreduras lineares, que podiam percorrer 10 milhões de
  linhas ocultas antes de reencontrar uma célula visível.
- Adicionados testes em `grade/eixos.rs` e
  `app/visibilidade_eixos.rs` para colunas consecutivas ocultas,
  congelamento, última dimensão e invalidação do cache após edição.

**Estado de validação:** código e testes gravados, mas não executados
(`cargo`/`rustc` indisponíveis nesta sessão). Nada disparado via
GitHub Actions, commits com `[skip ci]`. TypeScript histórico preservado
em `descarte/feathersheet/legacy-frontend/src/ui/grade/axis.ts`.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml rolagem_horizontal -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml visibilidade_eixos -j 2
```

## Incremento — 12/10/2026 — Grade: rolagem esparsa até 10 milhões de linhas

**Conferência de duplicidade:** o eixo esparso de `axis.ts` já estava
portado em `grade/eixos.rs` e utilizado no desenho/hit-test de linhas
ocultas em `grade/congelado.rs`. **Não foi criado outro eixo.** A lacuna
era `Grade::limite_inferior_rolagem` em `grade/auxiliares.rs`, que ainda
varria do final da planilha para trás uma linha por vez. Se as últimas
200 mil linhas estivessem ocultas, a rotina fazia 200 mil iterações **em
cada atualização da posição de rolagem**.

- `grade/eixos.rs`: `Eixo::inicio_da_cauda` calcula o primeiro índice
  rolável pelo prefixo esparso de alturas, em tempo logarítmico, sem
  percorrer as linhas que possuem altura zero. Respeita o detalhe da
  implementação anterior para empates de prefixo quando o viewport cabe
  exatamente nas últimas linhas.
- `grade/auxiliares.rs`: substituído o laço regressivo por chamada ao
  **mesmo cache `EstadoEixos`** usado pelo Canvas, preservando cálculo de
  altura física, zoom, cabeçalhos, linhas congeladas e o tratamento da
  última linha da planilha.
- `Eixo::anterior_visivel` complementa `proximo_visivel`: rolagem via
  roda do mouse salta blocos de linhas ocultas nas **duas direções**,
  em vez de exigir centenas de giros até achar um índice visível.
- Testes Rust escritos para faixa de 10 milhões com 200 mil últimas
  linhas ocultas, conjunto de **243 combinações** de alturas no eixo,
  casos de igualdade exata e salto regressivo/avançado. Uma simulação
  externa com 40 mil configurações aleatórias pequenas comparou o
  algoritmo binário ao laço anterior e encontrou equivalência, mas
  **isso não substitui** `cargo test`.

Arquivos TS **não tocados**; `descarte/feathersheet/legacy-frontend/src/ui/grade/axis.ts`
permanece como histórico. **Não houve execução de Cargo** neste ambiente
nem GitHub Actions (solicitação do usuário). Ainda falta validar
scroll horizontal e visual em Linux/Windows antes de considerar paridade total.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml limite_ -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml roda_ -j 2
```

## Incremento — 12/10/2026 — OpenQuery: seleção de múltiplas tabelas Web (TS → Rust)

**Conferência de duplicidade:** `importador::ler_web` já fazia download HTTP(S),
`html_tabela` já interpretava a primeira tabela e a janela Iced já oferecia
o campo Internet/API. A implementação nova reaproveita todos os três pontos,
sem outro cliente HTTP ou parser novo.

- `app/openquery_ui/importador.rs`: `html_tabelas` substitui a seleção
  exclusiva da primeira tabela. Enumera até **32 tabelas HTML**, sem executar
  JavaScript e com **160 mil células no total** entre todas as grades.
  Para JSON/CSV/TSV, o retorno é um vetor unitário e a carga segue automática.
- `app/openquery_ui.rs`: `TabelasWebLidas` recebe o resultado da tarefa
  de download; com múltiplas tabelas, mantém a fonte existente intacta enquanto
  aguarda `SelecionarTabelaWeb` e `ConfirmarTabelaWeb`. Ao confirmar, move
  a tabela escolhida, sem duplicar até 160 mil células; o cancelamento e a
  troca de URL invalidam o ticket antigo. O tratamento compartilhado de
  `ArquivoLido` permanece o único caminho que aplica a fonte e as etapas.
- `app/openquery_ui/janela.rs`: lista clicável com quantidade de linhas e
  colunas, indicação da tabela selecionada, prévia de **8 linhas × 6 colunas**,
  botões **Cancelar seleção** e **Importar tabela selecionada**. A lista de
  tabelas internas Ctrl+T não compete com a seleção de fonte externa.
- Atualização de URL compara URLs `reqwest::Url` normalizadas, para não
  perder etapas quando `https://dominio` vira `https://dominio/`.
- Novos testes para extração multi-HTML, limites agregados, seleção da segunda
  tabela, cancelamento, resposta assíncrona obsoleta, tabela única e
  reimportação sem perder etapas.

**Não executados** os testes locais (Cargo indisponível neste executor) e
**nenhum GitHub Actions** foi acionado por pedido do usuário. O arquivo TS de
origem permanece somente em `descarte/` para comparação histórica.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml html_ -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml web_ -j 2
```

## Incremento — 12/10/2026 — OpenQuery Web: paridade JSON/HTML com TypeScript

**Conferência prévia:** o botão Internet/API do editor Iced, o cliente HTTP(S)
assíncrono, `scraper` para HTML, parser CSV/TSV e parser JSON Rust **já existiam**.
Não foi criado outro conector ou dependência. A lacuna estava em
`descarte/feathersheet/legacy-frontend/src/openquery/ui/components/web-extractor.ts`:
`parseJSONWebData` aceitava formatos que `app/openquery_ui/importador.rs`
ainda rejeitava.

Migrados para o importador Rust existente:

- `{"data":[...]}` e `{"results":[...]}`: seleciona o primeiro campo que é
  lista, preservando a fonte externa em vez de tratar o envelope como registro.
- Arrays de valores simples e registros como listas; união das chaves dos
  objetos de uma coleção heterogênea, preenchendo dados ausentes com vazio.
- Campos JSON aninhados viram texto JSON serializado, sem avaliação de fórmula.
- Respostas JSON e HTML são reconhecidas por conteúdo mesmo quando um
  endpoint anuncia `text/plain` ou `application/octet-stream`.
- Tabelas HTML com cabeçalhos em `<td>` e `<th>` usam a primeira linha como
  cabeçalho; valores booleanos são reconhecidos como tais.
- Mantidos os limites existentes de **8 MiB por resposta Web** e
  **160 mil células por fonte**, com processamento HTTP fora da thread Iced.

Testes de código adicionados para envelope JSON, campo aninhado, listas de
primitivos, tabelas HTML com `<td>`, tipagem booleana e recusa de matrizes
muito grandes. **Não executados**: sem Cargo neste executor e sem GitHub
Actions por solicitação do usuário. O TypeScript histórico continua na pasta
`descarte`; nenhuma cópia ativa do parser foi recriada.

**Ainda pendentes de paridade:** HTML gerado dinamicamente por JavaScript,
paginação de APIs, autenticação OAuth e atualização agendada de fontes externas.
A seleção entre várias tabelas HTML foi portada no incremento subsequente.
Não declarar importação Web integralmente concluída sem compilar/testar localmente.

Validação local (não iniciar CI):

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml importador -j 2
```

## Incremento — 12/10/2026 — Credenciais de proteção de intervalo (TS → Rust)

**Conferência prévia:** `Sheet.pr` já atravessava a gravação e a leitura
FSHZ no Rust como JSON opaco, mas **nenhum módulo Rust derivava ou
conferia senhas PBKDF2**. O arquivo de origem
`descarte/feathersheet/legacy-frontend/src/ui/protecao/protecao.ts`
implementava toda a criptografia via `crypto.subtle`.

Portado para `feathersheet-gui/src/app/protecao_senha.rs`:
- `SenhaGuardada` serializável no formato **exato** do TypeScript
  (`alg`, `iteracoes`, `sal`, `hash`).
- `PBKDF2-HMAC-SHA256`, **600.000 iterações**, sal aleatório criptográfico
  de **16 bytes** (`getrandom`) e saída **32 bytes Base64**.
- Normalização **Unicode NFC** da senha, sem remover espaços finais.
- Conferência por comparação sem interrupção antecipada em 32 bytes.
- Validação defensiva de algoritmo, sal, hash e custo máximo declarado,
  antes de executar PBKDF2 em metadados oriundos de arquivo.
- Modelo tipado `ProtecaoDeIntervalo`/`AreaProtegida`, importação/exportação
  do JSON FSHZ, limites de dados e interseção de retângulos.
- Testes em código: vetor PBKDF2 conhecido, hash de 600 mil iterações,
  equivalência NFC, senha incorreta, metadados danificados e roundtrip
  com o leitor/gravadοr FSHZ Rust. **Não executados no GitHub.**

**IMPORTANTE: criptografia compatível não significa proteção ativa.**
Não habilitar ainda o comando visual `Proteger intervalo`. Restam os
diálogos, autenticação assíncrona e sobretudo **enforcement** de
intervalos protegidos **em todas as rotas de escrita** (edição, colagem,
macros, importadores), inclusive histórico e reexecução segura. Habilitar
um cadeado sem esse bloqueio permitiria editar dados supostamente protegidos.
Esta migração conclui somente a **camada de senhas e metadados**, não o
sistema completo de autorização.

A fonte TypeScript original já estava na pasta `descarte`; não
duplicamos a funcionalidade em `feathersheet-core`, que permanece sem
dependências criptográficas. Compilação local pendente; Actions não acionadas.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml protecao_senha -j 2
```

## Complementação — 12/10/2026 — Arraste e oito alças dos botões de macro

Auditoria antes da implementação: os objetos `Sheet.bt`, a renderização
Iced e o motor JavaScript já existiam. Faltavam especificamente os eventos
de mouse do `ui/grade/button-overlay.ts`: distinguir clique de arraste,
mover o objeto e redimensionar nas oito direções.

- Agora o `mouse_area` inicia o gesto, e o listener global de cursor/
  mouse-up (já usado nas imagens) acompanha o arraste. O clique só executa
  a macro ao soltar **sem cruzar 4 pixels**: mover jamais dispara o JS.
- A seleção do botão exibe as **oito alças**. O cálculo em pixels lógicos
  divide por zoom, usa mínimos de **40×24** e guarda somente ao soltar.
  A prévia do gesto não modifica `Sheet.bt` a cada evento.
- As alças e cursores são **reutilizados do módulo de imagens**; não foi
  criado um segundo mapa de cursores/direções.
- `Sheet::alterar_botoes` registra alterações de metadados como uma
  transação única no histórico Rust. Inclui criação, edição, exclusão,
  movimentação e redimensionamento, com Ctrl+Z/Ctrl+Y.
- **Delete/Backspace** exclui botão selecionado sem limpar a célula;
  **Escape** cancela gesto ou seleção; trocar de aba limpa a seleção.
- Mantém JSON compatível `bt` do TypeScript, inclusive campos extra dos
  itens trazidos do FSHZ. Há testes de arraste sem disparo de macro,
  clique com pequeno jitter, oito alças, desfazer/refazer e persistência.
- Não faz reexecução automática de macros na abertura do arquivo.

O fluxo principal de **criar / clicar / editar / mover / redimensionar /
apagar** saiu do TS. Ainda faltam validação visual local e compilação;
nenhum GitHub Actions ou teste remoto foi executado. Todos os commits
contêm `[skip ci]`.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento — 12/10/2026 — Botões de macro sobre a grade (Rust/Iced)

**Auditoria de duplicação:** o TypeScript legado possui
`ui/grade/button-overlay.ts` e `ui/caixas/botao-macro-dialog.ts`.
Em Rust, o motor Boa e o catálogo de módulos já existiam; o campo
`Sheet.bt` já fazia roundtrip no `.fshz`, mas era **JSON opaco** e
a Ribbon **Botão de macro** não tinha evento. O novo módulo
`app/botoes_macro.rs` reutiliza esses três recursos, sem duplicá-los.

Implementado:
- Inserir objeto de botão pela **Ribbon Macros**, ancorado à célula atual.
- Renderizar o botão no mesmo `stack` da grade e das imagens (considera
  rolagem, cabeçalho, zoom e painéis congelados), sem inserir célula falsa.
- **Clique** executa explicitamente o JavaScript atribuído usando
  `feathersheet_macros::executar` e seu rollback já existente. Nenhum
  código é executado durante renderização ou ao carregar o arquivo.
- Ícone **Editar** ao lado de cada botão abre modal Iced com rótulo e
  seleção entre módulos salvos. Salvar e **Excluir botão** não alteram
  a macro no catálogo global.
- Objetos são gravados no `Sheet.bt` com os nomes de campos
  `id`, `label`, `macroName`, `position` do TypeScript, para
  compatibilidade com FSHZ. Preserva campos extras de objetos existentes.
- Limite de 128 botões por aba; teste de roundtrip FSHZ e de execução
  apenas após clique inseridos no código.

**Na entrega inicial** faltavam arrastar e as 8 alças; estas foram
incluídas na complementação documentada acima. A validação visual e a
compilação local ainda são necessárias.

Não executamos GitHub Actions ou testes remotos. Os commits usam
`[skip ci]`.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento — 12/10/2026 — Localizar/Substituir com escopo da pasta em Rust

**Conferência prévia para não duplicar implementação:** o legado
`descarte/feathersheet/legacy-frontend/src/ui/caixas/find-replace-dialog.ts`
já tinha `findNext(reverse)`, `findAllMatches`, `replaceCurrent`, escopo
`sheet/workbook` e `replaceAllMatches`. O Rust **já implementava** o motor
`feathersheet-core/src/operacoes/ops.rs::find_all_com` /
`replace_all_em` e a GUI `app/dados.rs` tinha apenas Próxima,
Substituir tudo na aba e três opções de correspondência. Não foi criado
outro algoritmo de busca.

- A GUI agora oferece **Anterior/Próxima**, **Localizar todos**,
  **Substituir somente a célula ativa**, **Substituir tudo** e escopo de
  **todas as abas visíveis** (abas ocultas não recebem foco).
- A navegação entre abas usa o seletor nativo, mantém rolagem e seleção;
  `Ctrl+H` abre o painel de busca/substituição, `Ctrl+F` permanece.
- `Localizar todos` calcula a contagem e renderiza somente os primeiros
  **200 endereços clicáveis**. O motor `find_all_com` ainda materializa
  os achados **por aba**: esta entrega limita a UI, **não** o consumo de
  memória da busca em um conjunto muito grande. Busca de resultados
  paginada/limitada no motor é tarefa posterior.
- Ao substituir na pasta inteira, cada aba continua com histórico de
  desfazer **próprio**, sem prometer um Ctrl+Z global.
- Corrigida contagem de alterações do `replace_all_em`: casos em que o
  texto da fórmula não contém o termo, apesar de seu valor calculado
  corresponder à pesquisa, não são marcados como substituição efetuada.
- Testes adicionados para navegação reversa entre abas, aba oculta,
  listagem limitada, substituição da célula atual e undo por aba.

**Estado:** commits diretos com `[skip ci]`, sem executar Actions,
Cargo ou compilação remota. É necessária compilação local para validar
a interface e as chamadas do core.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento — 12/10/2026 — Criar módulo de macros JavaScript (Rust/Iced)

Antes, o editor de macros Rust já tinha o motor Boa, catálogo
`office.macros.v1` e ação **Guardar módulo**, mas os comandos **Novo módulo**
na Ribbon, na barra do editor, no menu **Inserir** e no botão `+` da aba
eram visuais/desabilitados. A criação de módulos agora passa pela
`App::criar_modulo_macro`:

- Sugere automaticamente `Modulo1`, `Modulo2`, ... sem conflito de nomes
  (comparação sem diferenciação de maiúsculas);
- Verifica se o código atual foi salvo, para **não sobrescrever rascunhos**;
- Valida os limites do catálogo compartilhado, grava atomicamente no
  armazenamento comum e só muda o editor após sucesso;
- Abre o novo módulo JavaScript sem executá-lo e mostra a janela ao invocar
  a ação pela Ribbon principal;
- Substitui botões sem ação por `Msg::MacroNovoModulo` na Ribbon, menu Arquivo,
  menu Inserir, barra de execução e botão `+` das abas;
- Adiciona testes de nomes únicos, impedimento de sobrescrita e menus reais.

Não foram implementados o criador de **componentes HTML/CSS**, formulários
visuais ou depuração passo a passo. A implementação JavaScript continua
sob o sandbox Boa do motor `featheroffice-macros`. O catálogo continua no
arquivo global do usuário, não embutido automaticamente no FSHZ.

**Validação:** testes escritos, porém não executados; nenhum workflow do
GitHub Actions foi iniciado. Para validação local:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento — 12/10/2026 — Reorganizar abas (Rust/Iced)

O `TabBar` TypeScript legado permitia selecionar, adicionar, renomear,
duplicar, colorir, ocultar e excluir guias, **mas não implementava
arrastar-e-soltar para reordenar**. O motor Rust já tinha `Workbook::mover_aba`;
a lacuna era o acesso pela GUI. Esta mudança complementa o gerenciamento
nativo de abas, sem reescrever o motor:

- Menu contextual de guias: **Mover aba à esquerda**, **à direita**,
  **para o início** e **para o fim** (somente ações possíveis).
- **Alt+Shift+PageUp/PageDown** move a guia ativa. Os comandos
  **Ctrl+PageUp/PageDown** navegam entre guias visíveis, ignorando ocultas.
- O índice ativo é recalculado a partir do **ID estável** depois da
  movimentação, sem misturar seleção, estilos, fórmulas ou congelamento.
- Durante edição de fórmula ou consulta OpenQuery, a mudança é bloqueada
  para não invalidar índices físicos de origem.
- A ordem é salva e restaurada por FSHZ sem introduzir um campo novo.
- Testes de código cobrem ordem, referências por nome, aba ativa, cores,
  guias ocultas, limites e roundtrip FSHZ. **Não executados**.

**Escopo:** o comando por menu e teclado está integrado. O gesto visual
de **arrastar a guia e soltá-la em outra posição** ainda NÃO foi
implementado; isto não é uma migração 1:1 de um gesto existente no TS,
pois o `TabBar` legado também não possuía arraste de guias. A compilação
local e a inspeção visual ainda são necessárias.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento — 12/10/2026 — Ocultar e reexibir linhas/colunas no Rust

Migrada a lógica de `descarte/feathersheet/legacy-frontend/src/ui/app/estrutura.ts`
para `feathersheet-core/src/planilha/sheet/edicao.rs` e
`feathersheet-gui/src/app/visibilidade_eixos.rs`:

- **Ocultar** grava largura/altura **0** no mapa de dimensões; **Reexibir**
  remove o tamanho 0 na seleção e nos blocos contíguos, restaurando o padrão.
- Cada comando cria **uma única transação desfazível** de dimensões no motor;
  nenhuma célula, fórmula, mescla ou referência é removida.
- Menu contextual dos cabeçalhos e grupo próprio na aba **Exibir**; clique
  direito sobre uma seleção de múltiplas linhas/colunas preserva o intervalo.
- Após ocultar o eixo da célula ativa, a seleção procura outro eixo visível.
- O formato FSHZ mantém zero nos metadados `h`/`w` preexistentes,
  com teste de round-trip. Não foi introduzido novo formato de arquivo.
- O mapa legível do legado tem uma entrada por índice. Para evitar consumo
  excessivo de RAM/histórico há **teto explícito de 200 mil dimensões por
  operação** até a migração futura para intervalos compactados.
- Testes de regressão escritos para undo/redo, largura personalizada,
  último eixo visível, fronteiras, faixa contígua e arquivo FSHZ.
  **Não executados: nenhuma ação do GitHub Actions foi iniciada.**

Validação local:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento — 12/10/2026 — Preencher células nativo (TBL-007/UI-010 [/])

A antiga ação de preenchimento da Ribbon da Página Inicial estava inativa, apesar de o
motor `feathersheet-core/src/operacoes/fill.rs` já suportar séries e fórmulas
relativas. O módulo novo `feathersheet-gui/src/app/preenchimento_ui.rs` liga esse
motor à GUI sem copiar a implementação TypeScript:

- Botão dividido **Preencher** e submenu **Baixo, Direita, Cima, Esquerda,
  Continuar até o fim**, usando a infraestrutura de popovers nativos.
- **Ctrl+D** e **Ctrl+R** na grade, com confirmação da edição ativa.
- Fonte/destino derivados da seleção e prévia sem escrita para o modo
  `AteOFim`; não preencher origem vazia nem regiões mescladas.
- Uma transação `Sheet::preencher` para que Ctrl+Z desfaça a operação.
- O motor ainda materializa o plano numa `Vec<FillCell>`: limite explícito
  de **250 mil células por comando** até existir execução incremental.

O gesto **arrastar a alça na grade** e o **duplo clique físico sobre a alça**
ainda exigem hit-test/Canvas Iced e não estão incluídos nesta entrega. A
complementação do gesto e o motor por lotes ficam pendentes para paridade
total; não declarar TBL-007 concluído. Testes de código adicionados para
deslocamento de fórmulas, quatro direções, sequência detectada na coluna
vizinha, origem vazia, mesclas e desfazer. **Sem execução Cargo/GitHub Actions**.

Validar localmente:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml preenchimento_ui -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento posterior — 12/10/2026 — Lembrar aba fixa da Ribbon no Rust/Iced

Migrado o comportamento de `descarte/feathersheet/legacy-frontend/src/ui/barras/ribbon.ts`
que guardava a última aba fixa em `localStorage`. A GUI Rust/Iced agora
restaura a aba em `App::novo_com` e persiste mudanças pelo próprio
`config-iced.json`, com identificadores estáveis (`arquivo`, `inicio`,
`inserir`, `dados`, `exibir`, `macros`, `ferramentas`).
A aba contextual `Tabela` nunca sobrescreve essa escolha. Configurações
anteriores sem a chave continuam válidas, usando `inicio`; chaves inválidas
também voltam para `inicio`. Abrir o catálogo de funções seleciona e lembra
`inserir`. Trocar de aba não modifica o Workbook, a flag de alterações ou
o histórico do documento.

Arquivos: `crates/feathersheet-gui/src/configuracao.rs`, `src/app.rs`,
`src/app/despacho_secundario.rs` e `src/app/testes.rs`.
Testes Rust escritos para round-trip de configuração, versão legada,
aba contextual e despacho. **Cargo e smoke visual não executados**;
o recurso permanece `P` no inventário até a validação real, sem GitHub Actions.

## Incremento posterior — 11/10/2026 — Congelar painéis nativo (APP-012/UI-009 `[/]`)

**Migrado em código para Rust/Iced:** botão **Exibir → Congelar** habilitado
com popover compacto para nenhuma/1/2 linhas, até a linha ativa, nenhuma/1/2
colunas, até a coluna ativa, congelar os dois eixos conforme a seleção e
descongelar. A árvore de comandos TypeScript `freeze`,
`freezeCurrentRow`, `freezeCurrentCol` passou a despachar a mesma
implementação, preservando o eixo não escolhido.

Arquivos:
- `app/congelar_ui.rs`: menu, mensagens, estado por ID estável de aba.
- `grade/congelado.rs`: cálculo de faixas fixas e roláveis **só no viewport**.
- `grade.rs` + `grade/auxiliares.rs`: pintura, números de linhas/letras de
  colunas, hit-test, seleção, menus dos filtros e mesclas sem percorrer
  o intervalo de milhões de linhas fora da vista.
- `app/despacho_mensagens.rs`, `app/despacho_secundario.rs`,
  `app/documento_operacoes.rs`: rolagem, PageUp/PageDown e navegação
  respeitam o limite da região fixa.
- `app/imagens_importadas.rs`: ancoragem visual das imagens no painel fixo
  usa a aproximação existente de larguras/alturas padrão.

A visualização congelada não replica células, não modifica a versão do
Workbook, não cria histórico de desfazer nem marca o documento como alterado.
O estado é conservado entre abas durante a sessão, e descartado ao abrir uma
pasta diferente. **Persistência do congelamento dentro do FSHZ ainda não foi
implementada.** Há limite explícito de 30 linhas e 16 colunas fixáveis; o
Canvas mantém ao menos 30% de espaço rolável, inclusive com zoom e dimensões
personalizadas. A renderização de mesclas e imagens que cruzam a linha divisora
ainda precisa de inspeção visual com exemplos reais.

Foram escritos testes de faixas congeladas, limites, colunas ocultas,
comandos de menu e independência do Workbook. **Ainda não executados**:
`cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2`
e `cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml congelado -j 2`.
Sem Rust/Cargo ou ambiente gráfico de teste neste executor, permanece
`[/]` até compilação e smoke Linux/Windows.

## Incremento posterior — 09/10/2026 — Exibir fórmulas (APP-012 `[/]`)

O inventário datado de **08/10/2026** abaixo permanece como linha de base.
A entrada **“Mostrar fórmulas” (N no snapshot)** passou a contar com implementação
de código no Rust/Iced em 09/10, mas **não com aceite funcional**: foi migrado
`toggleShowFormulas` do menu TS, exposto na Ribbon **Exibir → Fórmulas** e
associado ao atalho **Ctrl + crase** (o mesmo `Ctrl+&#96;` do TypeScript).
`Grade::texto` desenha o texto bruto da célula quando habilitado e
`Grade::alinhar` usa esquerda; a regra de formatação condicional “ocultar
valor” não esconde o conteúdo na inspeção, e os totais virtuais de tabela não
substituem a fórmula bruta. O modo normal continua avaliando valores.

Arquivos alterados: `feathersheet/crates/feathersheet-gui/src/{app.rs,grade.rs,app/faixa_nativa.rs}`.
Foram acrescentados testes para conteúdo, rascunho, estado do menu, atalho e
imutabilidade do documento. Commits
[`54c48d2`](https://github.com/M4R4G0N/FeatherOffice/commit/54c48d2b531f54ddb332a78325e6339de7e53897),
[`8e6c79a`](https://github.com/M4R4G0N/FeatherOffice/commit/8e6c79a099c6b7035e4c375914ccf6ee910444c5) e
[`4d991e9`](https://github.com/M4R4G0N/FeatherOffice/commit/4d991e916fb99e08791feca9ad63c67d5f2a2fce).

**Pendente:** `cargo check`, `cargo test`, smoke visual no Linux/Windows
(incluindo strings longas, mesclas e formatação condicional). O agente não
dispõe de Cargo/rustc. Portanto a mudança é de **N → P no estado de trabalho**,
e APP-012 continua `[/]`, sem alteração do total de itens `[x]`.

## Incremento posterior — 09/10/2026 — Perfil da coluna (APP-012 `[/]`)

A funcionalidade **“Perfil da coluna”**, inicialmente **M** (motor Rust existente, mas
sem frontend conectado), ganhou código de interface Rust/Iced: botão ativo em
**Dados → Ferramentas de dados** e ação `columnStats` do menu legado
(**Dados → Estatísticas da coluna**), diálogo de leitura com contagens de tipos,
vazias, distintos/repetidos, média, quartis, histograma e exemplos de outliers
com navegação à célula. Reusa diretamente
`feathersheet-core::operacoes::perfil::perfil_coluna`, sem materializar a grade
nem armazenar o resultado na planilha. Seleção de uma célula analisa a coluna
até a última linha usada; seleção vertical analisa somente esse intervalo.
Por segurança, a operação síncrona recusa intervalos acima de **100 mil linhas**
com mensagem explícita (sem amostragem ou truncamento silencioso); análise
assíncrona de colunas maiores permanece pendente.

**Escopo:** cálculo/semântica específicos do **FeatherSheet**;
janela/overlay seguem padrões Iced que podem ser extraídos para
`feather-ui` (**3 apps**) quando houver uso comum comprovado.
Testes foram adicionados no módulo GUI para escopo, outliers, navegação,
imutabilidade e limite. **Não houve aceite em Cargo nem smoke visual Linux/Windows**:
M → P no estado de trabalho, APP-012 segue `[/]`, sem mudar totais da linha
de base das 89 capacidades de 08/10.

Arquivo novo: `feathersheet/crates/feathersheet-gui/src/app/perfil_coluna_ui.rs`.
Integrações: `app.rs` e `app/faixa_nativa.rs`.

## Incremento posterior — 09/10/2026 — Redesenho do botão Menu (UI-005/APP-012 `[/]`)

O botão **☰ Menu** agora abre um painel tipo *backstage* inspirado no
protótipo visual, independentemente da aba **Arquivo**, que permanece
inalterada. A navegação lateral tem **Início**, **Novo**, **Abrir**,
**Recentes**, **Salvar**, **Salvar como**, **Fazer uma cópia**, **Importar**,
**Exportar Excel**, **Informações**, **Opções**, **Comandos** e **Fechar**.
Imprimir e Compartilhar ficam visualmente indisponíveis enquanto não houver
implementação funcional — sem despachos fictícios. O menu de nove categorias
legado foi preservado em **Comandos**, com navegação hierárquica e teclado.

A tela inicial exibe cartões Novo/Abrir/Recentes e arquivos realmente
abertos ou salvos. O histórico utiliza `feathersheet-io::recents`, persiste
em `feathersheet/recents-iced.json` sob o diretório de configuração do usuário,
permite abrir, fixar, desfixar e retirar arquivos do histórico; caminhos
inexistentes não são abertos. Uma escolha de recente permanece fixa mesmo
quando o documento atual precisa ser salvo antes da abertura. **Fazer uma cópia**
grava um FSHZ separado sem substituir o caminho do documento atual. O botão
**Fechar** fecha o documento (novo em branco) com a confirmação existente de
alterações não salvas. O menu é um overlay e não reduz a área da grade.

**Escopo:** estrutura do painel, cartões, listas, botões e gestão de recentes
candidatos a `feather-ui`/3 aplicativos; codecs FSHZ, Workbook, confirmação
de saída e ações de célula exclusivos do Sheet. **Código implementado, testes
de estado escritos; sem confirmação de compilação Cargo ou smoke visual.**
UI-005 e APP-012 permanecem `[/]` até validação real.

Arquivos: `feathersheet/crates/feathersheet-gui/src/app/menu_principal.rs`
(novo) e `feathersheet/crates/feathersheet-gui/src/app.rs`.

## Incremento posterior — 11/10/2026 — Exportar JSON e Fazer uma cópia (APP-011/APP-012 `[/]`)

Migrou-se do frontend TypeScript a ação `exportar('json')` para o GUI
Rust/Iced: **Arquivo → Exportar JSON**, **Ribbon Arquivo → JSON .json** e
**menu legado Arquivo → Exportar → Pasta completa (.json)**.
O formato segue as chaves JSON v2 (`version`, `active`, `sheets`,
`names`, `cells`, `raw`, `style`, dimensões e metadados).
O novo `feathersheet-io/src/json_workbook.rs` percorre as três
origens de células nativas (avulsas, colunares, fórmulas por padrão)
sem criar `Workbook::toJSON`, `Vec<FormulaEntry>` ou matriz densa.
O arquivo é escrito por `BufWriter` de 64 KiB em temporário com
substituição atômica, preservando o arquivo antigo em falhas.

Também foi ligado **Fazer uma cópia** da Ribbon ao comando de cópia FSHZ
já existente e mapeado o ID TS `makeCopy` no menu nativo, evitando
criar uma segunda implementação de salvamento.

Testes de código verificam JSON parseável (duas abas, fórmula, Unicode,
metadados, célula distante esparsa e propagação de erro), além do
despacho dos IDs de menu TS para mensagens Rust.
**Pendente:** executar `cargo test -p feathersheet-io`,
`cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2`,
smoke visual e medição de RAM/tempo com arquivos grandes. O processo
de exportação permanece síncrono na GUI e pode bloquear temporariamente
a interface em pastas grandes. A propriedade `oculta` de aba, sem par
persistido em `Sheet` nativo, não é emitida no JSON; validar demais
metadados com fixtures reais antes de afirmar paridade integral.
APP-011/APP-012 seguem `[/]`, não `[x]`.

## Incremento posterior — 11/10/2026 — Nomes de intervalos e comandos existentes (APP-012 `[/]`)

Migradas as ações TypeScript `nameSelection`, `manageNames` e o comportamento
da **caixa de nomes** (não apenas endereço A1:C10). O módulo
`app/nomes_ui.rs` fornece gerenciador nativo Rust/Iced para criar, listar,
navegar e remover intervalos, validando nomes (até 60 caracteres; não aceita
endereços de células). A lista mostra a aba e a referência de cada nome,
permite ir ao intervalo inclusive em outra aba e fechar com Esc. O menu
**Dados → Nomes** e as ações legadas do TypeScript despacham para o motor
`Workbook::definir_nome/remover_nome/nomeados`, sem cópia das células.
Criar/remover marca alterações pendentes, permitindo persistir os nomes via
FSHZ. A navegação não marca o arquivo como alterado. Os testes escritos
abrangem validação, nomes duplicados, seleção, exclusão e referência
indisponível; **não foram executados por falta de Rust/Cargo no ambiente**.

A renomeação de uma aba no `feathersheet-core` foi ajustada para migrar as
referências `Rect.sheet` dos nomes, além das referências das fórmulas já
reescritas anteriormente; teste de regressão verifica o valor de
`=SOMA(Receitas)` ao renomear Dados → Relatório.

Além disso, foram ativados os botões já existentes **Arquivo → Recentes**
(menu backstage) e **Dados → Tabela dinâmica** (editor Rust já integrado),
sem copiar seus motores.

**Pendente:** `cargo test` do core e GUI, `cargo check`, smoke visual e
validação com arquivos importados que possuam nomes/abas internacionais.
APP-012 permanece `[/]`, sem afirmar paridade integral da interface.

## Incremento — 11/10/2026 — OpenQuery nativo (APP-012 [/] e APP-011 [/])

O motor Rust em `feathersheet-core/src/consulta/query.rs` já continha os passos de
transformação do TypeScript. A migração ativou `Dados → OpenQuery` e a ação do
catálogo `openOpenQuery`, acrescentando editor nativo Iced no módulo
`feathersheet-gui/src/app/openquery_ui.rs`. A primeira linha do intervalo
selecionado é interpretada como cabeçalho; clicar numa célula isolada utiliza
a área usada da aba. O editor permite renomear consulta, selecionar colunas
para operações, visualizar amostra, editar/remover etapas, acrescentar outra
aba como fonte de join/append e carregar a saída em uma **nova aba**, sem
reescrever a origem. Há ações diretas para filtro, ordenação, tipo de dado,
renomeação, projeção, distintos, limite, índice, erros, separar linhas, mesclar
colunas, agrupar, despivotar, juntar à esquerda, acrescentar e manipular
cabeçalhos. Etapas avançadas, como pivotar, podem ser descritas no JSON,
compatível com os valores primitivos dos passos do TypeScript.

O executor protegido `executar_query_limitado` foi acrescentado ao motor sem
alterar a assinatura original `executar_query`. O preflight em
`consulta/query/limites.rs` calcula linhas e colunas antes de cada etapa,
inclusive a multiplicação de JOIN muitos-para-muitos, divisão de linhas,
agrupamento, pivotagem, despivotagem e append. A GUI recusa previamente
mais de **160 mil células por fonte** e **240 mil células na saída**,
sem truncar silenciosamente. **Não é streaming**: aumentar esses limites
não é seguro sem um novo motor por lotes ou colunares.

**Ainda faltam para paridade total:** persistir/atualizar consultas em
`Sheet.oq` e vínculos das tabelas, múltiplas consultas e suas pastas,
UI de merge/pivot mais avançada, importadores CSV/Excel/web/API/SQL dentro
do editor, atualização agendada, scripts gerados, editor M, snapshots,
cancelamento/progresso de execução e testes de memória em milhões de linhas.
A implementação é **parcial** e o TypeScript não foi removido.

Testes unitários Rust acrescentados para leitura do intervalo, bloqueio de
seleção excessiva, importação tipada, passos, join um-para-um e explosivo,
despivotagem e round-trip do JSON TypeScript. **Não executados neste ambiente
(sem Cargo/rustc)**, e GitHub Actions não ofereceram execução de etapas para
o commit inicial. Manter APP-012 e APP-011 `[/]` até compilação,
testes e inspeção visual.

Comandos locais:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-core/Cargo.toml limites -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml openquery_ui -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento posterior — 11/10/2026 — Hiperlinks nativos e Ribbon (APP-011/APP-012 `[/]`)

O TypeScript `src/ui/caixas/link-dialog.ts` e `src/ui/grade/link.ts` teve
fluxo migrado para `feathersheet-gui/src/app/link_ui.rs`: janela Iced com
endereço e texto, preservação do conteúdo de fórmula, validação de URL
(`http`, `https`, `mailto`, `www.`), remoção do campo `Style.link` sem
apagar o texto e abertura via lançador do sistema operacional sem shell.
Protocolos `javascript:` e `file:` são recusados antes de executar o lançador.
A gravação de texto e estilo é feita em uma única transação de desfazer.
O endereço continua nos metadados de estilo do motor nativo, não há coluna
ou cache alternativo de links.

Integração visual: **Inserir → Link** habilitado, ações Abrir/Remover,
**menu contextual de célula**, ID TypeScript `editarLink`, tecla Esc,
Canvas com cor de destaque e sublinhado para hiperlinks explícitos ou
textos que são endereços completos, incluindo células mescladas; **Ctrl+clique**
abre a URL da célula apontada. O menu **Nomear intervalo** em Inserir
também foi ligado à migração anterior, sem reimplementar o motor de nomes.

Testes adicionados: bloqueio de esquemas perigosos, reconhecimento
de URL em texto, alteração de texto e link/desfazer, fórmula inalterada e
validação do diálogo. **Ainda não executados**: `cargo check`,
`cargo test`, smoke visual em Linux/Windows e validação do lançador
de links com desktops sem `xdg-open`. A paridade completa com links do
XLSX, incluindo todos os gestos de mouse, depende de testes reais.
APP-011/APP-012 permanecem `[/]`.

## Incremento — 11/10/2026 — OpenQuery: salvar e atualizar consulta no FSHZ (APP-012 [/] / APP-011 [/])

O editor Iced ganhou lista de consultas persistidas e comandos **Editar** e **Atualizar (↻)**. O módulo
`app/openquery_ui/registro.rs` armazena o mesmo contrato `ConsultaItem` do TypeScript
(`id`, `nome`, `fonte`, `passos`, `destino`, `status`) em `Sheet.oq`, campo
que os codecs FSHZ já preservam. Um objeto complementar `_feather` contém
origem, intervalo, fontes adicionais e **retângulo de saída**. O registro
preserva outras consultas existentes e detecta metadados inválidos sem
apagá-los. IDs de consultas anteriores são mantidos.

O módulo `app/openquery_ui/execucao.rs` relê a origem e fontes extras
ao atualizar, executa o pipeline com preflight, atualiza **na aba vinculada**
e limpa somente a área do resultado anteriormente gerado; outros valores
da planilha não são apagados. A atualização não duplica a aba. Quando o
arquivo TS não contém `_feather.areaSaida`, **não substitui** células
sem saber com certeza o destino; a consulta pode ser aberta pelo snapshot
para inspeção/edição, mas não atualizada sobre a planilha legada. A primeira
gravação nativa cria uma aba e estabelece o vínculo. A UI mantém a relação
de consultas em memória durante a edição, sem reanalisar `Sheet.oq` a cada
desenho do Iced.

**Limites de segurança ainda existentes:** motor vetorial, máximo de 160 mil
células por fonte / 240 mil por resultado; o OpenQuery permanece síncrono.
A carga tipada da consulta não implementa undo/redo do resultado; para não
deixar um Ctrl+Z parcialmente válido, o histórico da aba gerada é
limpo ao atualizar. O histórico das demais abas não é alterado.
Persistência da consulta no arquivo ocorre ao **salvar o FSHZ** pelo
comando já existente do aplicativo, não imediatamente ao clicar em Carregar.
Não foi implementada atualização automática periódica.

**Testes escritos (não executados):** consultas legadas preservadas,
atualização sem duplicar aba, limpeza de sobra sem alterar células externas,
falha segura para origem removida e **round-trip .fshz real** usando
`feathersheet_io::da_workbook::pasta_gravar_bytes` seguido por
`fshz_read::ler_fshz` e `para_workbook`. Cargo/rustc indisponíveis no
ambiente de edição; não marcar [x] antes da validação Linux e Windows.
Comandos: `cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2`
e `cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml openquery_ui -j 2`.

## Resumo verificável

- **262 arquivos `.ts/.tsx/.css`** em `feathersheet/src/` no inventário do Git. **150 de frontend/inicialização** conforme as pastas `ui/`, `ajuda/`, `monitor/`, `editor-macros/`, `openquery/ui/`, `configuracoes/`, `componente/`, `idiomas/` e `main.ts`. Os **112 restantes** são majoritariamente motores, modelos, IO, ponte e runtime TS: são dependências da migração, *não* tarefas de UI 1:1.
- **89 funcionalidades/capacidades auditadas e agrupadas**, inventário de **4 blocos**. Estado de código: **A 4** (existe explicitamente), **P 61** (parcial), **M 19** (motor Rust localizado, GUI inexistente ou não comprovada), **N 5** (não localizada no frontend Rust).
- O menu TS apresenta nove grupos (Arquivo, Editar, Ver, Inserir, Formatar, Dados, Macros, Ferramentas e Ajuda) e catálogo atual com **147 entradas com ação**, incluindo Pintar linha/coluna e Colar somente formatação acrescentadas à árvore original. A presença no JSON **não implica função ativa**. A GUI já exibe o catálogo e liga parte das ações; outras permanecem desabilitadas.
- Os IDs `UI-`, `APP-`, `TBL-`, `CHT-`, `MAC-`, `IMG-`, `IO-`, `QA-` abaixo são do **checklist oficial**, não novas tarefas abertas. **Nenhuma linha deste inventário foi marcada `[x]` por gerar o relatório.** A maior parte permanece `[/]` aguardando testes Cargo/smoke.
- **Destino arquitetural proposto:** **10 capacidades de interface diretamente compartilháveis pelos três**, **23 específicas do FeatherSheet** e **56 de implementação mista** (widget/motor comum com lógica e dados exclusivos do Sheet). Esses totais classificam *dono do código futuro*, não afirmam que cada capacidade já funciona nos 3 produtos.
- **Priorização de lacunas:** **24 P0**, **53 P1**, **12 P2**. P0 significa alto impacto, não que todos os seus módulos estejam 100% ausentes; alguns são correção/regressão de algo parcial.

- **A/P/M/N não significa teste aprovado.** É diagnóstico a partir de *código fonte e chamadas*; não há aqui garantia de compilação, integração completa, execução Linux/Windows ou pixel parity. Mesmo A necessita smoke para encerrar demanda.

### Regra de escopo (muito importante)

| Classificação | O que é compartilhável | O que não deve virar código comum |
|---|---|---|
| **3** / **3 apps** | Widget Iced, cromo, Ribbon/menu, popup, tooltip, estado selecionado, atalhos globais, campos/botões, color picker, abas visuais, ajuda, diálogo de arquivo, progresso e monitor; motores independentes de documento (Drawing, Image, Charts, Tables, IO, Macros). | *Não* copiar semântica de célula, intervalo, fórmula ou estrutura de pasta para Docs/Slide. |
| **Sheet** / **exclusivo** | Grade de 10 milhões de linhas, editor de célula/fórmula, planilhas/abas do workbook, tabela dinâmica, filtro de células, validação, nomes, OpenQuery de planilha, vínculos, formato FSHZ/XLSX, modelos de objetos ancorados à célula, perfil/estatística por coluna. | Não forçar `feather-ui`, `featheroffice-tables` ou `featheroffice-charts` a substituir a engine esparsa/colunar do Sheet. |
| **3 (widget), Sheet (ação)** | Renderização, hit-test, seleção visual e layout em crate neutro; comando/estado de domínio no FeatherSheet por mensagens tipadas. | Um checkbox 3 não significa que a funcionalidade completa terá que existir também em Slide e Docs. |

**Critério de compartilhamento:** extrair para `feather-ui` apenas interface pura `Element<Message>` + callback; extrair para crates neutros apenas algoritmos sem Iced e sem `Sheet`; deixar adaptadores e estados de negócio em `feathersheet-gui`. Priorizar reutilizar implementações já existentes — **não migrar TS linha por linha**.

### Legenda e prioridades

**A** = implementado em código Rust visível; **P** = parcial/sem paridade demonstrada; **M** = motor existente no Rust mas não exposto completamente em UI; **N** = interface/comando ausente ou não localizado. **P0** = usabilidade/bloqueio; **P1** = paridade funcional; **P2** = avançado. Alguns recursos P podem inclusive ter cobertura robusta recente; verificar mudanças concorrentes antes de abrir implementação.

### 01 — Shell, menus, Ribbon, busca, comandos e configurações

| Funcionalidade TypeScript | Origem TS | Situação no Rust atual | Paridade | Destino arquitetural | Checklist oficial relacionado | Prioridade |
|---|---|---|:---:|---|---|:---:|
| Shell Tauri/WebView → janela Iced, título e controles | `ui/app/app.ts; ui/barras/topbar.ts` | GUI app.rs + feather-ui::cromo | **A** | 3 | UI-001/UI-012 | P1 |
| Barra de menu com 9 categorias, submenus e breadcrumbs | `ui/barras/menu.ts; menus.ts` | GUI menu_nativo.rs + app.rs; 147 entradas catalogadas, despacho parcial | **P** | 3 (menu), Sheet (ações) | UI-005/APP-012 | P0 |
| Hover entre menus, fechamento fora, posicionamento/foco e scroll | `ui/barras/menu.ts` | GUI navega clique/teclado; hover e comportamento exato sem smoke | **P** | 3 | UI-004/UI-009/UI-010 | P1 |
| Busca global de comandos e atalhos na Ribbon | `ui/barras/menus.ts; toolbar.ts` | GUI faixa_nativa.rs resultados_faixa e app.rs BuscaComandos | **P** | 3 (controle), Sheet (catálogo) | UI-005/UI-007/APP-012 | P1 |
| Ribbon completa por abas/grupos com ícones | `ui/barras/ribbon.ts; ribbon-galeria.ts` | GUI faixa_nativa.rs + feather-ui::componentes | **P** | 3 | UI-003/UI-005/APP-012 | P0 |
| Ribbon contextual de tabelas (estilo, totais e filtros) | `ui/barras/ribbon-tabela.ts` | GUI tabelas/painel.rs sem paridade contextual comprovada | **P** | Sheet (ferramenta); 3 (modelo contextual) | TBL-013/APP-012 | P1 |
| Ribbon contextual de tabelas dinâmicas | `ui/barras/ribbon-dinamica.ts` | GUI sem editor dinâmico localizado | **M** | Sheet (Ribbon contextual reaproveitável) | APP-012 | P2 |
| Galerias de estilos de tabela e prévias | `ui/barras/ribbon-galeria.ts` | GUI tabelas.rs/painel.rs parcial; sem galeria equivalente | **P** | 3 (galeria), Sheet (estilos de tabela) | UI-006/TBL-013 | P1 |
| Barra compacta alternativa e overflow inteligente | `ui/barras/toolbar.ts` | GUI Ribbon recolhível e botões; overflow dinâmico não equivalente | **P** | 3 | UI-005/UI-009 | P1 |
| Aba Ribbon ativa memorizada | `ui/barras/ribbon.ts` (em `descarte/feathersheet/legacy-frontend/src/`) | **Código migrado em 12/10:** GUI `configuracao.rs` persiste chave estável da aba fixa; `app.rs` restaura na abertura; `app/despacho_secundario.rs` grava mudanças e acesso ao catálogo. Aba contextual Tabela não substitui a fixa. Testes escritos, sem Cargo executado. | **P** | 3 | UI-007/APP-012 | P2 |
| Menus suspensos, checkmarks, atalhos e disabled dinâmico | `ui/barras/menu.ts; menus.ts` | GUI item_menu_marcado e item_menu_disponivel; nem todos os estados TS | **P** | 3 (widget), Sheet (ações) | UI-004/UI-005 | P1 |
| Temas claro/escuro e cor de acento | `ui/estilo/theme.ts; palettes.ts; tema-janela.ts` | feather-ui::tema + GUI configuracao.rs | **A** | 3 | UI-001/UI-008 | P1 |
| Cores, fontes, bordas, popovers e tooltips | `ui/barras/toolbar.ts; ui/estilo/styles.ts` | feather-ui::formatacao + GUI estilos_comuns.rs; popovers incompletos | **P** | 3 (widgets), Sheet (style) | UI-006/UI-011/APP-010 | P1 |
| Diálogos genéricos (alerta, confirmar, prompt, modal) | `ui/caixas/dialog.ts; ui/app/caixas.ts` | Iced janelas/painéis; sem paridade integral de diálogos | **P** | 3 | UI-006/UI-009 | P1 |
| Teclas globais, foco, F1/F12, IME e cancelamento | `ui/app/teclas-globais.ts; keybindings.ts` | feather-ui::atalhos + GUI app.rs; IME/foco exigem testes reais | **P** | 3 (atalhos), Sheet (teclas de célula) | UI-007/UI-010 | P0 |
| Preferências aparência, salvamento, planilha, consultas e atalhos | `ui/paineis/settings-panel.ts; ui/app/montar-configuracoes.ts` | GUI configuracao.rs + view_configuracoes; seções TS avançadas ausentes | **P** | 3 (núcleo), Sheet (opções) | UI-007/UI-008 | P1 |
| Idiomas/locales UI e tradução completa | `idiomas/index.ts; idiomas/registro.ts` | GUI ajuda.rs com locais; tradução completa da Ribbon não confirmada | **P** | 3 (infra i18n), Sheet (textos) | UI-008/UI-013 | P2 |
| Barra de status, zoom, alertas e métricas da seleção | `ui/barras/statusbar.ts` | **Implementado (código, validação de compilação pendente):** core `sheet.rs::contar_selecao` e GUI `app/status_selecao.rs` com Soma/Média/Mín/Máx/Contagem, filtro de linhas ocultas, cache por versão/seleção, processamento de grandes áreas em lotes e formatação pt-BR; percentual do zoom clicável (100%). Permanecem a conferir erro ativo clicável e paridade visual/performance real. | **P** | 3 (status), Sheet (agregações) | UI-005/APP-012 | P1 |
| Notificações temporárias, progresso e erros acessíveis | `ui/paineis/loading-overlay.ts; ui/app/limite-memoria.ts` | GUI status/monitor; sem overlay progressivo/cancelável equivalente | **P** | 3 | IO-005/QA-009/UI-009 | P0 |
| Página inicial, documentos recentes e recuperação | `ui/paineis/start-page.ts; io/recents.ts` | IO recents.rs motor; GUI sem página inicial completa | **M** | 3 (launcher/recents), Sheet (FSHZ) | UI-005/APP-011 | P1 |
| Arquivos recentes: abrir, remover, localizar e metadados | `io/recents.ts; ui/paineis/start-page.ts` | IO recents.rs existente; Ribbon Recentes sem callback | **M** | 3 (componente), Sheet (recentes) | APP-011/UI-005 | P1 |
| Painel de configurações ligado ao app + atalhos | `configuracoes/principal.ts; ui/app/configuracoes-ponte.ts` | GUI Configuracoes e preferências locais, ponte TS não migrada inteiramente | **P** | 3 | UI-007/UI-008 | P2 |

### 02 — Grade, seleção, fórmulas, edição e desempenho visual

| Funcionalidade TypeScript | Origem TS | Situação no Rust atual | Paridade | Destino arquitetural | Checklist oficial relacionado | Prioridade |
|---|---|---|:---:|---|---|:---:|
| Canvas de células esparso, cabeçalhos e zoom | `ui/grade/grid.ts; desenho.ts; desenho-celulas.ts` | GUI grade.rs com canvas/células/cabeçalhos | **A** | Sheet (grade); 3 (draw primitives) | APP-012/DRW-004 | P0 |
| 10 milhões de linhas, viewport virtual e scroll | `ui/grade/vista.ts; axis.ts; mouse.ts` | GUI grade.rs/medidas.rs com limite MAX_ROWS e virtualização | **P** | Sheet | UI-009/QA-007 | P0 |
| Geometria de linha/coluna personalizada e DPR/scroll | `ui/grade/axis.ts; zoom.ts; desenho-cabecalho.ts` | GUI grade.rs + medidas.rs; paridade zoom/DPI incompleta | **P** | Sheet (geometria), 3 (DPI) | UI-009/UI-012/TBL-008 | P0 |
| Seleção simples, intervalos, cabeçalhos e múltiplos gestos | `ui/grade/mouse.ts; eventos.ts; grid.ts` | GUI grade.rs + selecao.rs; casos avançados e touch sem aceite | **P** | Sheet (modelo); 3 (pointer/drag infra) | UI-010/TBL-007 | P0 |
| Seleção por teclado, Ctrl+seta, Shift e aba | `ui/grade/teclado.ts; ui/app/keybindings.ts` | GUI app.rs + edicao_formula.rs; core ref_drag.rs | **P** | Sheet | UI-010/TBL-007 | P0 |
| Ctrl+F4/F2, edição de referências e autocomplete | `ui/grade/f4.ts; assist-popup.ts; edicao.ts` | GUI edicao_formula.rs e campo_fx.rs, sugestões e navegação | **P** | Sheet (fórmulas); 3 (autocomplete) | UI-004/UI-010 | P1 |
| Editor inline na célula (edição in-place, IME, seleção de texto) | `ui/grade/edicao.ts; grade/grid.ts` | GUI barra fx e teclado; editor inline TS integral não identificado | **P** | Sheet (editor); 3 (IME) | UI-010/APP-012 | P0 |
| Barra fx, caixa de nome, seleção por endereço e referências | `ui/barras/formula-bar.ts; ui/app/estrutura.ts` | GUI campo_fx.rs + app.rs IrPara/assistente de função | **P** | Sheet | APP-012/UI-010 | P1 |
| Referência cruzada com seleção de outra aba | `ui/grade/edicao.ts; ui/app/ciclo.ts` | GUI edicao_formula.rs; cenários ainda precisam de teste manual | **P** | Sheet | APP-012/UI-010 | P1 |
| Histórico de undo/redo e transações de estilo/estrutura | `ui/app/historico-estrutura.ts; core/ops.ts` | GUI app.rs, Workbook core e testes | **P** | Sheet (histórico); 3 (modelo undo UI) | TBL-007/QA-004 | P0 |
| Recortar/copiar/colar e área múltipla (TSV, estilos) | `ui/grade/transferencia.ts` | GUI clipboard + Snap + Colar Valores/Formato/Pincel | **P** | Sheet (células), 3 (clipboard UI) | TBL-007/APP-012 | P0 |
| Colar especial valores/formatação sem trocar fórmulas | `ui/barras/menus.ts; ui/grade/transferencia.ts` | GUI handlers ColarValores/ColarFormato; recém ligados ao menu | **P** | Sheet | APP-012 | P0 |
| Arrastar borda para mover/copiar e prevenir sobrescrita | `ui/grade/arrastar.ts` | GUI grade.rs + app.rs; implementação mas testes finais pendentes | **P** | Sheet (operações); 3 (gestos) | UI-010/TBL-007 | P0 |
| Alça preencher séries, fórmulas relativas e duplo clique | `ui/grade/arrastar.ts; ui/grade/preencher.ts` | core operacoes/fill.rs implementado; GUI gesto parcial | **P** | Sheet | TBL-007/UI-010 | P1 |
| Autoscroll durante drag e seleção | `ui/grade/arrastar.ts; mouse.ts` | GUI grade.rs; casos de borda/zoom não aceitos | **P** | Sheet | UI-009/UI-010 | P1 |
| Linhas e colunas ocultas, reexibir e redimensionar | `ui/app/estrutura.ts; ui/grade/axis.ts` | core reflow.rs + GUI grade.rs/parcial; menu sem fluxo completo | **P** | Sheet | TBL-008/APP-012 | P1 |
| Congelar painéis por linha e coluna | `ui/grade/vista.ts; ui/barras/menus.ts` | nenhum comando de congelar conectado no GUI app.rs | **N** | Sheet (semântica), 3 (modelo split view) | UI-009/APP-012 | P1 |
| Mesclar/desmesclar células e impactos em navegação/cópia | `ui/app/estrutura.ts; core/mesclagem.ts` | core mesclas; UI Iced sem ação de menu confirmada | **M** | Sheet | TBL-006/TBL-013/APP-012 | P0 |
| Formatos, alinhamentos, bordas e autoajuste | `ui/barras/toolbar.ts; ui/grade/desenho-celulas.ts` | GUI faixa_nativa.rs + estilos_comuns.rs, bordas_comuns.rs | **P** | 3 (widgets de estilo), Sheet (células) | UI-006/APP-010 | P1 |
| Formatação condicional (regras, prévia, edição e prioridades) | `ui/condicional/conditional-panel.ts` | core operacoes/rules.rs + GUI condicional.rs e app.rs | **P** | Sheet (regras), 3 (painel genérico) | APP-015/UI-006 | P1 |
| Faixas alternadas e realce linha/coluna persistente | `ui/grade/desenho-celulas.ts; ui/barras/menus.ts` | GUI condicional + configuracao.realce_eixos | **P** | Sheet (regra visual); 3 (preferências) | APP-015/UI-007 | P2 |
| Mostrar fórmulas (alternar texto vs valor) | `ui/barras/menus.ts; ui/grade/desenho-celulas.ts` | GUI menu contém comando TS não conectado | **N** | Sheet | APP-012 | P2 |
| Cor/estilo de erro + explicação guiada por célula | `ui/caixas/error-explainer.ts; ui/grade/edicao.ts` | core formula/explicar.rs existe; modal Iced não localizado | **M** | Sheet (explicação); 3 (modal) | UI-013/APP-012 | P2 |

### 03 — Abas, tabelas, dados, validação, impressão e arquivo

| Funcionalidade TypeScript | Origem TS | Situação no Rust atual | Paridade | Destino arquitetural | Checklist oficial relacionado | Prioridade |
|---|---|---|:---:|---|---|:---:|
| Abas: criar, selecionar, renomear, excluir | `ui/barras/tabbar.ts; ui/app/estrutura.ts` | GUI view_abas, app.rs, renomear_aba.rs | **A** | Sheet (abas); 3 (tabs visuais) | APP-012/UI-005 | P0 |
| Abas: duplicar, ocultar, reexibir e cores | `ui/barras/tabbar.ts; ui/app/estrutura.ts` | GUI contexto/cor da aba APP-016 parcial; duplicar/ocultar não comprovados | **P** | Sheet | APP-016/APP-012 | P1 |
| Menu contextual de célula/linha/coluna/aba | `ui/app/botao-direito.ts` | GUI app.rs contexto; posição/paleta de aba em APP-016 | **P** | Sheet (ações); 3 (popover) | APP-016/UI-009 | P1 |
| Tabelas formatadas: criar, editar nome, estilo e faixas | `ui/app/tabelas.ts; ui/barras/ribbon-tabela.ts` | GUI tabelas.rs + tabelas/painel.rs cria e desenha, edição parcial | **P** | Sheet (metadados), 3 (galeria) | TBL-013/APP-012 | P1 |
| Tabelas: linha total, converter em intervalo, filtros de coluna | `ui/app/tabelas.ts; ui/caixas/filter-popup.ts` | GUI painel.rs filtros/cabeçalhos; nem todos os controles TS disponíveis | **P** | Sheet | TBL-013/APP-012 | P1 |
| AutoFiltro geral e dropdown cabeçalho avançado | `ui/app/filtro.ts; ui/caixas/filter-popup.ts` | core operacoes/filtro.rs + GUI tabelas/painel.rs; Ctrl+Shift+L | **P** | Sheet | APP-012/TBL-013 | P0 |
| Ordenação de intervalo, página e filtros avançados | `ui/barras/menus.ts; ui/app/acoes-do-menu.ts` | GUI dados.rs Ordenar intervalo; página e critérios TS incompletos | **P** | Sheet | APP-012 | P1 |
| Remover duplicados, aparar espaços, autoSoma | `ui/app/acoes-do-menu.ts; ui/caixas/duplicados-dialog.ts` | Novo diálogo Rust/Iced `app/duplicados_ui.rs`: expande região de dados, detecta cabeçalho, seleciona colunas por checkbox, marcar/desmarcar todas, confirmar/cancelar; `ops::planejar_repetidas`/`aplicar_repetidas` mantidos no core. Testes adicionados, **Cargo ainda não executado** (toolchain ausente no ambiente). Aparar espaços e AutoSoma continuam em dados.rs. | **P** | Sheet | APP-012 | P1 |
| Dividir texto em colunas com assistente por etapas | `ui/caixas/dividir-texto-dialog.ts` | core operacoes/ops.rs suporta operações; GUI assistente não localizado | **M** | Sheet (algoritmo), 3 (wizard) | APP-012/UI-006 | P1 |
| Validação de dados (listas, mensagens, erro, data, número) | `ui/caixas/validacao-dialog.ts` | core operacoes/validation.rs tem regras; GUI formulário ausente | **M** | Sheet (regras), 3 (form UI) | APP-012 | P0 |
| Perfil da coluna: quartis, histograma, outliers e frequência | `ui/caixas/perfil-coluna.ts` | core operacoes/perfil.rs completo; GUI sem painel identificado | **M** | Sheet (perfil), 3 (cartões/gráficos) | APP-012 | P1 |
| Nomes de intervalos e caixa de nome com gestão | `ui/app/estrutura.ts; ui/caixas/nome-do-intervalo.ts` | GUI navegação por endereço; gerenciar/editar nomes não localizado | **P** | Sheet | APP-012 | P1 |
| Links/hiperlinks: criar, editar, abrir com validação | `ui/caixas/link-dialog.ts; ui/grade/link.ts` | GUI faixa_nativa botão Link inativo; IO XLSX pode conter hyperlinks | **M** | Sheet (células); 3 (diálogo de link) | APP-011/APP-012 | P1 |
| Referências externas entre pastas e atualização | `ui/app/ciclo.ts; modelo/refs.ts` | core planilha/vinculo.rs implementado; GUI sem fluxo completo | **M** | Sheet | APP-011/IO-010 | P2 |
| Proteção de intervalos/edição com senha e desbloqueio | `ui/protecao/*; ui/caixas/validacao-dialog.ts` | GUI faixa Ferramentas botão Proteção sem integração identificada | **N** | Sheet (restrições); 3 (dialog de senha) | APP-012 | P1 |
| Criar/abrir/salvar FSHZ/XLSX/CSV/TSV | `ui/app/abrir-arquivo.ts; ui/app/gravar.ts` | GUI abrir/salvar e XLSX; IO FSHZ/XLSX/CSV em crates; cobertura parcial | **P** | 3 (diálogo/IO), Sheet (formatos) | APP-011/IO-005/IO-008 | P0 |
| Exportar CSV/TSV/JSON e cópia da pasta | `ui/app/gravar.ts; ui/barras/menus.ts` | **CSV e TSV implementados (sem cargo executado):** botão Arquivo → Exportar e ações do menu TS; exporta apenas aba ativa, fórmulas pelo valor, escapa aspas/delimitadores, escrita atômica por streaming sem alterar o FSHZ. JSON e demais fluxos ainda pendentes. | **P** | Sheet (serialização), 3 (export wizard) | APP-011/IO-010 | P1 |
| Imprimir, pré-visualização e seleção/área de impressão | `ui/caixas/print-dialog.ts; ui/grade/impressao.ts` | GUI botões Impressão/Imprimir desativados | **N** | 3 (print host), Sheet (paginação/fórmulas) | IO-011/APP-011 | P1 |
| Salvar automático, cópia de segurança, recuperação de sessão | `ui/app/gravar.ts; io/backup-policy.ts; ui/paineis/start-page.ts` | GUI salvar manual + prompts; política recovery/autosave não localizada | **P** | 3 (backup/recovery UI), Sheet (FSHZ) | IO-012/APP-011/QA-009 | P0 |
| Recentes e página inicial (histórico/pins/remover) | `ui/paineis/start-page.ts; io/recents.ts` | IO recents.rs pronto; GUI ainda sem tela integrada equivalente | **M** | 3 (launcher), Sheet (formatos) | UI-005/APP-011 | P1 |
| Abertura massiva com progresso/memória/abortar | `ui/paineis/loading-overlay.ts; io/files.ts` | IO streaming CSV e limites Rust; GUI sem overlay progressivo equivalente | **M** | 3 (progress dialog), Sheet (carga esparsa) | IO-005/QA-009 | P0 |

### 04 — Tabela dinâmica, consultas, gráficos, imagens e automação

| Funcionalidade TypeScript | Origem TS | Situação no Rust atual | Paridade | Destino arquitetural | Checklist oficial relacionado | Prioridade |
|---|---|---|:---:|---|---|:---:|
| Tabela dinâmica: criação por assistente, campos/valores | `ui/dinamica/pivot-dialog.ts; pivot-panel.ts` | core dinamica/pivot.rs completo; GUI não tem editor visual | **M** | Sheet (configuração); 3 (arrastar campos UI) | APP-012 | P0 |
| Tabela dinâmica: arrastar campos, medidas, data, filtros, refresh | `ui/dinamica/*; ui/barras/ribbon-dinamica.ts` | core dinâmica + layout; painel/contexto Iced ausentes | **M** | Sheet | APP-012 | P1 |
| Tabela dinâmica: grid próprio, drilldown, formatar e campo calculado | `ui/dinamica/grid-dinamica.ts; pivot-panel.ts` | core dinamica/campo_calculado.rs; sem frontend equivalente | **M** | Sheet | APP-012 | P1 |
| OpenQuery editor de consultas, etapas, prévia e ribbon | `openquery/ui/modal.ts; step-pane.ts; preview-grid.ts` | GUI Iced em `app/openquery_ui.rs`, prévia seletiva por etapa, reordenação/remoção de passos e `Sheet.oq`; edição avançada ainda parcial | **M** | Sheet (pipeline); 3 (wizard/tabelas) | APP-012 `[/]` | P0 |
| OpenQuery: importar web/CSV/tabela/valores, merge/append | `openquery/ui/components/*` | Importador assíncrono CSV/TSV/TXT/JSON/XLSX; assistentes Iced de merge (6 modos/multichave), append e pivot; web/SQL/entrada manual ainda pendentes | **M** | Sheet (carga/ETL) | APP-011/APP-012 `[/]` | P1 |
| OpenQuery: editor avançado, M-like scripts, árvore de passos | `openquery/ui/components/editor-avancado.ts; openquery/core/script-generator.ts` | Editor JSON compatível com valores TS e árvore de etapas em Iced; gerador de script M-like e editor de expressões completos ainda pendentes | **M** | Sheet | APP-012 `[/]` | P1 |
| Gráficos: criar, escolher tipos, mover/alterar tamanho | `ui/grafico/chart.ts; chart-dialog.ts` | GUI graficos.rs, posicionamento.rs, seletor_tipo.rs com 25 tipos | **P** | 3 (Charts), Sheet (objetos/âncoras) | CHT-015/CHT-018/APP-009 | P0 |
| Gráficos: painel editar dados, séries, intervalos e tipo | `ui/grafico/chart-panel.ts` | GUI graficos/editor.rs parcial; paridade das opções avançadas não provada | **P** | 3 (editor de estilos), Sheet (fonte) | CHT-017/CHT-018 | P1 |
| Gráficos: paleta, eixos, série secundária, 3D, tooltip, data labels | `ui/grafico/chart-panel.ts; chart.ts` | featheroffice-charts + editor Iced; opções TS ainda parcialmente migradas | **P** | 3 (render/edição), Sheet (dados) | CHT-006..012/CHT-017/CHT-018 | P1 |
| Gráficos: interação hover/hit-test, zoom, copiar/duplicar | `ui/grafico/chart.ts` | GUI graficos.rs eventos; paridade clipboard e hover não comprovada | **P** | 3 (hit-test/Charts), Sheet (objetos) | CHT-012/CHT-018 | P1 |
| Gráficos: projetos, financeiro e famílias especializadas | `ui/grafico/chart.ts; chart-panel.ts; linha-3d.ts` | GUI seletor de 25 tipos e Charts comum, render todos requer CHT-017 | **P** | 3 (Chart families), Sheet (agregações) | CHT-009/CHT-017/CHT-018 | P1 |
| Imagem: inserir, selecionar, mover, redimensionar, gravar FSHZ | `ui/grade/image-overlay.ts` | GUI imagens_importadas.rs com PNG/JPEG, borda e alça ↘ | **P** | 3 (controles de objeto), Sheet (coordenadas/FSHZ) | IMG-014/UI-010 | P1 |
| Imagem: 8 alças, lock ratio opcional, recorte e contexto | `ui/grade/image-overlay.ts` | GUI só uma alça proporcional; sem paridade de 8 alças | **P** | 3 (gestos), Sheet (âncora) | IMG-007/IMG-010/IMG-014 | P1 |
| Imagem: excluir, copiar/colar, colar do SO e drag/drop arquivo | `ui/grade/image-overlay.ts` | GUI edição de pixels e import; clipboard/drag-drop de imagem não localizados | **P** | 3 (clipboard/drop), Sheet (objetos) | IMG-014/UI-010 | P1 |
| Imagem: efeitos não destrutivos/histórico por objeto | `ui/grade/image-overlay.ts; modelo/imagem.ts` | GUI Efeitos última imagem; Image engine existe; histórico/Undo não comprovado | **P** | 3 (efeitos), Sheet (objeto) | IMG-010/IMG-014 | P1 |
| Macros: motor e ações em documento | `macros/runtime.ts; macros/api-*.ts` | featheroffice-macros/feathersheet-macros + GUI macros.rs | **P** | 3 (runtime genérico/host), Sheet (API) | MAC-001 | P1 |
| Editor VBA-like: árvore, menus, abas, propriedades e terminal | `editor-macros/pagina.ts; desenhador.ts; propriedades.ts` | GUI macros/editor_visual.rs estrutura extensa migrada | **P** | Sheet (experiência VBE), 3 (editor genérico) | MAC-002 | P1 |
| Macro debugger: breakpoint, step, watch, console e mensagens | `editor-macros/pagina.ts; macros/depurador.ts` | GUI editor_visual.rs controles; execução/inspeção de debug parcial | **P** | 3 (debug UI), Sheet (host) | MAC-001/MAC-002 | P1 |
| Editor HTML/CSS de componentes e desenhador visual | `editor-macros/desenhador.ts; ui/caixas/componente-dialog.ts` | GUI editor_visual.rs parcial, runtime componentes do legado não comprovado | **P** | Sheet (componente de planilha), 3 (form designer opcional) | MAC-002 | P2 |
| Botões de macro sobre a grade e associação a eventos | `ui/grade/button-overlay.ts; ui/caixas/botao-macro-dialog.ts` | GUI não possui overlay de botões ligado à planilha confirmado | **N** | Sheet | MAC-001/MAC-002 | P2 |
| Ajuda F1, busca, tópicos, atalhos e funções | `ajuda/*; ui/app/ajuda.ts` | feather-ui::ajuda + GUI ajuda.rs, ajuda_macros.rs integrados | **P** | 3 (busca/render), Sheet (conteúdo) | UI-013 | P1 |
| Monitor CPU/memória, histórico e painel | `monitor/*; ui/paineis/memory-panel.ts` | GUI view_monitor + pulso de métricas; árvore memória não equivalente | **P** | 3 (monitor), Sheet (métricas) | UI-007/QA-007 | P2 |
| Console de logs/erros e área técnica | `ui/paineis/debug-console.ts` | GUI view_monitor parcial; console histórico detalhado TS não encontrado | **P** | 3 | UI-007/QA-009 | P2 |

## Onde já há código Rust reutilizável — não reimplementar

| Área | Reuso comprovado por símbolo/arquivo | Uso correto |
|---|---|---|
| Menu/Ribbon/ícones | `feather-ui/src/ribbon.rs`, `componentes.rs`, `icones.rs`; GUI `app/menu_nativo.rs` e `faixa_nativa.rs` | Generalizar o *controle*, manter IDs/args e dispatcher locais no Sheet. **UI-005/APP-012** |
| Ajuda | `feather-ui/src/ajuda.rs`; GUI `app/ajuda.rs`, `app/ajuda_macros.rs` | Busca/navegação para 3, textos próprios por app. **UI-013** |
| Estatísticas de colunas | `feathersheet-core/src/operacoes/perfil.rs::perfil_coluna` | Criar painel Iced consumindo métricas; não recalcular em widget. |
| Filtros | `feathersheet-core/src/operacoes/filtro.rs` + GUI `app/tabelas/painel.rs` | Evoluir UI Excel-like e ligar autoFiltro sem escrever nova engine. |
| Validação de entrada | `feathersheet-core/src/operacoes/validation.rs` | Construir diálogo Iced, aplicar regra via core e persistir. |
| Tabela dinâmica | `feathersheet-core/src/dinamica/pivot.rs`, `campo_calculado.rs` e layout | Portar painel/campos/drag; engine já existe. |
| OpenQuery | `feathersheet-core/src/consulta/query.rs` (`Passo`, `executar_query`) | Portar wizard de passos, prévia e conectores com isolamento/cancelamento. |
| Preenchimento | `feathersheet-core/src/operacoes/fill.rs` (`plan_fill`, `preencher`, duplo clique) | Concluir eventos visuais e undo; não duplicar progressões. |
| Vínculos de pasta | `feathersheet-core/src/planilha/vinculo.rs` | Criar fluxo de escolher/atualizar outra pasta e erros de arquivo. |
| Gráficos | `featheroffice-charts`, `feathersheet-io/src/grafico_compartilhado.rs`, GUI `app/graficos/*` | Desenho comum nos 3, agregação e ancoragem de células no Sheet. |
| Imagens | `featheroffice-image`, GUI `app/imagens_importadas.rs` | Operações e editor compartilháveis, FSHZ/posição exclusivos. |
| Tabelas | `featheroffice-tables` + `feathersheet-core/src/operacoes/tabela.rs`, GUI `app/tabelas/*` | Reusar estilos/algoritmos, **não** materializar milhões de células em Tables. |
| Recents | `feathersheet-io/src/recents.rs` | Só falta fluxo visual completo; componente de launcher pode ser comum. |
| Macros | `featheroffice-macros` + GUI `app/macros/editor_visual.rs` | Motor com host por app; VBE Sheet específico até prova de generalização. |
| Exportação | `feathersheet-io` FSHZ/XLSX/CSV + `featheroffice-io` | Conectar formatos não expostos, progressos, confirmação de perda. |

## Cruzamento com o checklist — o que já foi feito × o que não foi

**Crates/infra já marcados `[x]` não são prova de paridade frontend:** `UI-001`, `UI-002`, `FND-*`, `CHT-004`, `CHT-005`, `TBL-004`, `TBL-005`, `IO-001..004` e outros fundamentos. Eles permanecem ativos como dependências e **não precisam ser refeitos**.

| Pacote oficial | Situação no checklist (conferir `main`) | Cobertura neste inventário |
|---|---|---|
| **UI-003 a UI-013** | `[/]` no snapshot | Shell, menus, popovers, tooltips, ajuda, acessibilidade, mouse, foco, DPI, portabilidade, layout e testes. |
| **APP-009/010/011/012/015/016** | `[/]` no snapshot | Sheet Charts, estilos/bordas, arquivo, Ribbon, condicional, menu contextual e aba. |
| **TBL-006..013** | `[/]` no snapshot | Arrasto/seleção/estrutura de tabelas, mesclas, edição, filtros e estilos. |
| **CHT-006..009/012/014..018** | `[/]` no snapshot (alguns testes com fontes comuns) | Editor/hit-test/gráficos avançados e motor de gráficos; **CHT-018** trata explicitamente renderizadores `chart.ts`. |
| **IMG-007/008/010/013/014** | `[/]` no snapshot | 8 alças/crop/histórico/efeitos e integração de imagem; requisitos comuns não são exclusivos do Sheet. |
| **MAC-001/002** | `[/]` no snapshot | Runtime Boa e editor VBE. **MAC-002 tem estrutura Iced**, não confundir com depurador funcional completo. |
| **IO-005..014** | variam entre `[/]` e `[x]`; consultar | Streaming, salvamento seguro, exportação/round-trip e alertas de perda. |
| **QA-001/002/004/005/009/010/012** | `[/]` no snapshot | Compilação, regressão, comparação visual, acessibilidade e validação real antes de marcar concluído. |

### Proposta de ordem de execução para não gerar retrabalho

1. **P0 — confiabilidade e gestos básicos:** executar Cargo; corrigir erros, smoke de menu/Ribbon/grade/clipboard, abertura/edição FSHZ e UI de carregamento; validar IME, edição inline e undo. **Compartilhado:** foco, diálogos, estados, notificações. **Exclusivo:** grade e fórmulas.
2. **P0 — conectar motores já presentes:** validação de dados; painel de pivot; OpenQuery; autoFiltro; editor de fórmulas/células, sem reescrever core. **Compartilhado:** formulários, listas, wizards. **Exclusivo:** lógica das operações.
3. **P1 — paridade do trabalho cotidiano:** imprimir e exportar, recentes/recovery, congelar painéis, mesclar, ocultar/reexibir, gerenciamento de intervalos, hiperlinks, submenus avançados. Dividir UI comum e modelo Sheet.
4. **P1 — objetos:** concluir 8 alças, clipboard, efeitos/undo; charts editor completo, 25 famílias, seleção/hover e export; preservar posição/dados FSHZ. **Compartilhado:** Drawing/Image/Charts; **exclusivo:** âncoras e agregação.
5. **P2 — editor avançado/macros/suporte:** depurador, componentes, proteção, console técnico, tradução completa; QA Windows/Linux com capturas reais.

## Anexo A — cobertura de arquivos de frontend TS

**150 arquivos identificados** e enumerados abaixo, agrupados por pasta real. É a lista de entrada para auditorias específicas e evita esquecer arquivos menores (tooltip, vista, arquivo, contexto, filtros, desenho, temas). A coluna de paridade é por **grupo de funcionalidades nas tabelas acima**, não veredito por arquivo.

| Pasta relativa a `feathersheet/` | Arquivos | Nomes |
|---|---:|---|
| `src/` | 1 | `main.ts` |
| `src/ajuda/` | 6 | `ajuda-css.ts`, `conteudo.ts`, `pagina.ts`, `pedido.ts`, `principal.ts`, `secoes.ts` |
| `src/componente/` | 1 | `principal.ts` |
| `src/configuracoes/` | 1 | `principal.ts` |
| `src/editor-macros/` | 10 | `css.ts`, `desenhador.ts`, `inspecao.ts`, `opcoes.ts`, `pagina.ts`, `pontos.ts`, `principal.ts`, `procedimentos.ts`, `propriedades.ts`, `realce.ts` |
| `src/idiomas/` | 2 | `index.ts`, `registro.ts` |
| `src/monitor/` | 7 | `aba-console.ts`, `aba-memoria.ts`, `aba-metricas.ts`, `metricas-texto.ts`, `monitor-css.ts`, `pagina.ts`, `principal.ts` |
| `src/openquery/ui/` | 3 | `modal.ts`, `openquery-css.ts`, `types.ts` |
| `src/openquery/ui/components/` | 8 | `append-wizard.ts`, `editor-avancado.ts`, `enter-data.ts`, `import-wizard.ts`, `merge-wizard.ts`, `preview-grid.ts`, `step-pane.ts`, `web-extractor.ts` |
| `src/ui/app/` | 29 | `abrir-arquivo.ts`, `abrir.ts`, `acoes-do-menu.ts`, `ajuda.ts`, `app-dialogs.ts`, `app-openquery.ts`, `app.ts`, `botao-direito.ts`, `caixas.ts`, `ciclo.ts`, `comandos.ts`, `comum.ts`, `configuracoes-ponte.ts`, `estrutura.ts`, `eventos-de-macro.ts`, `filtro.ts`, `gravar.ts`, `historico-estrutura.ts`, `keybindings.ts`, `limite-memoria.ts`, `monitor-ponte.ts`, `montar-barras.ts`, `montar-configuracoes.ts`, `montar-grade.ts`, `operacoes.ts`, `tabelas.ts`, `teclas-globais.ts`, `tema.ts`, `vinculo-janelas.ts` |
| `src/ui/barras/` | 13 | `formula-bar.ts`, `function-menu.ts`, `icons.ts`, `menu.ts`, `menus.ts`, `ribbon-dinamica.ts`, `ribbon-galeria.ts`, `ribbon-tabela.ts`, `ribbon.ts`, `statusbar.ts`, `tabbar.ts`, `toolbar.ts`, `topbar.ts` |
| `src/ui/caixas/` | 15 | `botao-macro-dialog.ts`, `componente-dialog.ts`, `dialog.ts`, `dividir-texto-dialog.ts`, `duplicados-dialog.ts`, `error-explainer.ts`, `filter-popup.ts`, `find-replace-dialog.ts`, `guia-macros.ts`, `link-dialog.ts`, `nome-do-intervalo.ts`, `perfil-coluna.ts`, `print-dialog.ts`, `range-picker.ts`, `validacao-dialog.ts` |
| `src/ui/condicional/` | 2 | `condicional-acoes.ts`, `conditional-panel.ts` |
| `src/ui/dinamica/` | 6 | `dinamica-opcoes.ts`, `grid-dinamica.ts`, `pivot-dialog.ts`, `pivot-dialogs.ts`, `pivot-filtros.ts`, `pivot-panel.ts` |
| `src/ui/estilo/` | 5 | `janela-feather.ts`, `palettes.ts`, `styles.ts`, `tema-janela.ts`, `theme.ts` |
| `src/ui/grade/` | 24 | `alvos.ts`, `arrastar.ts`, `assist-popup.ts`, `axis.ts`, `button-overlay.ts`, `desenho-cabecalho.ts`, `desenho-celulas.ts`, `desenho-painel.ts`, `desenho.ts`, `edicao.ts`, `eventos.ts`, `f4.ts`, `grid-css.ts`, `grid-tipos.ts`, `grid.ts`, `image-overlay.ts`, `impressao.ts`, `link.ts`, `mouse.ts`, `preencher.ts`, `teclado.ts`, `transferencia.ts`, `vista.ts`, `zoom.ts` |
| `src/ui/grafico/` | 6 | `chart-dialog.ts`, `chart-panel.ts`, `chart.ts`, `linha-3d.ts`, `lttb.ts`, `medir-grafico.ts` |
| `src/ui/paineis/` | 7 | `debug-console.ts`, `functions-panel.ts`, `loading-overlay.ts`, `macro-panel.ts`, `memory-panel.ts`, `settings-panel.ts`, `start-page.ts` |
| `src/ui/protecao/` | 4 | `protecao-controle.ts`, `protecao-dialog.ts`, `protecao.ts`, `protecoes-lista.ts` |

## Anexo B — arquivos TS dependentes (motor/modelo/IO/ponte)

Os **112 arquivos** abaixo integram a aplicação TypeScript, mas **não representam widgets frontend 1:1**. A maioria já possui crate Rust ou parte de migração concluída; precisam ser checados por **paridade de comportamento**, não por cópia. `openquery/core`, `macros`, `core`, `modelo`, `io` e `ponte` aparecem porque são chamados por telas TS.

| Pasta | Arquivos | Nomes |
|---|---:|---|
| `src/core/` | 42 | `assist.ts`, `ast.ts`, `async.ts`, `campo-calculado.ts`, `chart.ts`, `column-store.ts`, `data-table.ts`, `date-serial.ts`, `evaluator.ts`, `explain.ts`, `fill.ts`, `fn-helpers.ts`, `fn-lookup-matrix.ts`, `fn-lookup.ts`, `fn-math.ts`, `fn-stats.ts`, `fn-text.ts`, `fn-web.ts`, `formula-pattern.ts`, `formula-table.ts`, `formula-text.ts`, `function-help.ts`, `functions.ts`, `input.ts`, `mesclagem.ts`, `ops.ts`, `parser.ts`, `pivot.ts`, `pool.ts`, `query.ts`, `ref-drag.ts`, `rules.ts`, `sheet-deps.ts`, `sheet-query.ts`, `sheet-spill.ts`, `sheet.ts`, `style.ts`, `table.ts`, `tokenizer.ts`, `validation.ts`, `values.ts`, `workbook.ts` |
| `src/io/` | 10 | `backup-policy.ts`, `delimited.ts`, `files.ts`, `fshz-arrow.ts`, `fshz.ts`, `gunzip-worker.ts`, `metadados-aba.ts`, `recents.ts`, `xlsx.ts`, `zip.ts` |
| `src/macros/` | 24 | `api-aplicacao.ts`, `api-contexto.ts`, `api-dados.ts`, `api-estrutura.ts`, `api-formato.ts`, `api-navegacao.ts`, `api-pasta.ts`, `api-valores.ts`, `armazenamento.ts`, `canal-componente.ts`, `canal-editor.ts`, `componentes.ts`, `conversao.ts`, `depurador.ts`, `desenho.ts`, `enderecos.ts`, `eventos.ts`, `fn-planilha.ts`, `funcoes.ts`, `guia.ts`, `instrumentar.ts`, `motor-antigo.ts`, `pedido-editor.ts`, `runtime.ts` |
| `src/modelo/` | 22 | `aba.ts`, `botao-macro.ts`, `celula.ts`, `consulta.ts`, `dinamica.ts`, `estilo.ts`, `explicacao.ts`, `formula.ts`, `grafico.ts`, `imagem-base64.ts`, `imagem.ts`, `locale.ts`, `mapa-com-versao.ts`, `mesclas.ts`, `motor-antigo.ts`, `operacoes.ts`, `pasta.ts`, `refs.ts`, `regras.ts`, `tabela.ts`, `validacao.ts`, `valores.ts` |
| `src/openquery/` | 2 | `index.ts`, `standalone.ts` |
| `src/openquery/core/` | 1 | `script-generator.ts` |
| `src/ponte/` | 10 | `bloco.ts`, `canal.ts`, `consulta.ts`, `csv.ts`, `formula.ts`, `index.ts`, `metricas.ts`, `pasta-rust.ts`, `planilha-rust.ts`, `tarefas.ts` |
| `src/workers/` | 1 | `csv-loader.worker.ts` |

## Verificação automática da cobertura do inventário

O utilitário [`auditar_frontend_ts_rust.py`](../../scripts/auditar_frontend_ts_rust.py)
não modifica a planilha ou o repositório. Ele enumera todos os
`*.ts/*.tsx/*.css` de `feathersheet/src/` e compara **arquivo por arquivo**
com os anexos A (150) e B (112); também confere duplicatas, total das
89 capacidades por status (A/P/M/N) e se os IDs mencionados
**existem realmente no checklist oficial**.

```bash
python3 feathersheet/scripts/auditar_frontend_ts_rust.py --check --json
```

**Critério esperado:** `"status": "ok"` e código de saída `0`;
qualquer novo arquivo TS sem classificação, ID desconhecido ou total divergente
deve retornar `1` e exigir atualização manual do relatório. Esse script
verifica **cobertura documental, não compilação/correção da GUI Rust**.
A verificação estrutural foi também confrontada diretamente contra a árvore
Git da `main` em 08/10/2026: 262 TS, 150 frontend, 112 suporte,
89 capacidades e 42 IDs de tarefas referidos existentes.

## Definition of done do mapeamento

Este documento inventaria funcionalidades/arquivos e identifica *lacunas presumidas*. Para marcar qualquer linha como **paridade comprovada**: (1) confirmar evento Rust Iced e ação na UI; (2) `cargo check --all-targets`; (3) `cargo test` abrangendo estado/undo/persistência; (4) abrir GUI Linux e Windows; (5) screenshot comparável com TS e casos de zoom/DPI; (6) regressão FSHZ/XLSX e macro quando aplicável; (7) só então atualizar a demanda oficial de `[/]` para `[x]`. Não usar quantidade de módulos Rust ou testes escritos como percentual de funcionalidade finalizada.

**Atualização concorrente:** o repositório possui outras frentes editando a `main`. Antes de iniciar item desta tabela, buscar SHA/checkout mais recente e ler o checklist; se outro agente avançou a GUI depois da auditoria, atualizar **este arquivo**, sem reabrir a mesma demanda.

11/10/2026 — OpenQuery (em andamento, **não marcar [x]**): continuação com assistentes Iced de junção esquerda/interna/direita/total/anti-esquerda/anti-direita, chaves múltiplas, append e pivot com agregação; importação local CSV/TSV/TXT/JSON/XLSX em tarefa separada e com limites; proteção para tarefas obsoletas; prévia por etapa e alteração de ordem; seleção de coluna por índice além das sete visíveis; persistência FSHZ + vínculo `Sheet.tb`/`openQueryId`/`openQueryNome`, intervalo nomeado, proteção contra colisão ao crescer e testes de regressão. A tela principal foi refatorada para separar `openquery_ui/testes.rs` e continuar abaixo de 1.000 linhas. **Pendências de paridade:** PowerQuery/linguagem M-like e editor visual avançado completo, web HTML/API, conectores SQL, entrada manual, modos de destino (aba atual/apenas conexão), streaming e cancelamento/progresso acima de milhões de células, recarga explícita de arquivos externos, comparação visual Linux/Windows e compilação Cargo. O fluxo do TSV/CSV também precisa de testes de valores e metadados após reabertura. Consulte `docs/funcionalidades/OPENQUERY-RUST-PARIDADE.md`.
