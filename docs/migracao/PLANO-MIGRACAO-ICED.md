# FeatherSheet — plano de migração integral do frontend para Rust + Iced

> **Auditoria consolidada (08/10/2026):**
> [89 funcionalidades/capacidades comparadas TS × Rust, com status,
> prioridade e escopo de reutilização 3 apps vs. exclusivo do Sheet](AUDITORIA-FRONTEND-TS-RUST-3APPS.md).
> A auditoria atual cruza implementações e checklist oficial; **este plano
> anterior continua válido como fases e requisitos**, mas descrições antigas
> como “não implementado” devem ser confrontadas com o código mais recente da
> `main` antes de abrir outra demanda. Nenhum `[/]` vira `[x]` pela auditoria.


> **Escopo:** substituir a interface Tauri/WebView/TypeScript pela interface Iced nativa; **não reescrever os motores Rust já existentes**. `feathersheet-core`, `feathersheet-io`, `feathersheet-ponte` e `feather-ui` são as referências.  
> **Branch de trabalho:** `main`, conforme solicitado. **Nada de branch/PR para cada tarefa.**  
> **Estado do documento:** plano; itens marcados como pendentes **não são funcionalidades já entregues**.  
> **Objetivo:** equiparar, testar e só então desativar a interface antiga; preservação integral de arquivos é condição obrigatória.

## Situação real da migração para Rust/Iced — 08/10/2026

> Este quadro é de **paridade do frontend FeatherSheet**, não do motor nem
> da quantidade de funções na planilha. `Em implementação` não significa
> aceite concluído. Os IDs abaixo continuam com checkbox vazio até haver
> teste automatizado, compilação, revisão visual e aceite integral do plano.
>
> A GUI Iced executa sem Tauri/WebView; o frontend TypeScript continua no
> repositório como referência e para funcionalidades ainda sem paridade.

| Fase | Estado comprovado no código da `main` | Ainda para aceitar / implementar |
| --- | --- | --- |
| 0. Infraestrutura | **Parcial:** crates Rust, janela nativa Iced, componentes `feather-ui`, testes e CI existentes | Estabilizar todos os builds, clippy/fmt, proteção de arquivos complexos e matriz de aceite Linux/Windows |
| 1. Seleção/teclado | **Avançado, mas parcial:** âncora e intervalos, mouse, cabeçalhos, atalhos, referência visual e barra fx | Foco/IME e testes manuais em ambas as plataformas; bordas e casos extremos |
| 2. Arrasto/viewport | **Parcial:** mover/copiar pela borda, alça, resize/autoajuste, parte do auto-scroll e transações | Congelar painéis, rolagem completa, tabbar avançada, todos os gestos/aceites |
| 3. Menus/Ribbon | **Parcial:** menus tipados e contexto, sete abas na Ribbon, cores, temas, F1/F12, atalhos, dropdowns compartilhados e funções pesquisáveis | Paridade com todos os comandos TypeScript, hover/submenus completos, ações desabilitadas, layout responsivo e testes de GUI |
| 4. Fórmulas/estilo | **Parcial:** autocomplete fx, referências entre abas, paleta, fontes/formatos, copy/paste interno de estilos | Editor inline real, IME, mesclar, condicional, validação, colar especial e testes visuais |
| 5. Dados/arquivo | **Parcial:** FSHZ/XLSX/CSV, salvar/abrir, procurar, ordenar, deduplicar e abas/linhas/colunas básicas | Filtros completos, grandes arquivos, impressão/PDF, recentes, preservação de objetos e recovery |
| 6. Ferramentas avançadas | **Não migrado integralmente:** existem motores Rust, mas não os editores Iced equivalentes | Gráficos, tabelas dinâmicas, formatação condicional, OpenQuery, validação e painéis |
| 7. Macros | **Parcial:** há interface/editor e runtime Rust inicial | Compatibilidade do legado, API, segurança, sandbox, integração/testes completos |
| 8. Entrega final | **Parcial:** já roda em Rust/Iced no Linux | Benchmark, acessibilidade, instaladores Linux/Windows e remoção segura do legado |

**Dropdown de funções (incremento atual):** o menu de funções do FeatherSheet
passou a usar `iced::widget::combo_box`, com pesquisa e seleção por teclado;
os nomes canônicos e apelidos PT-BR vêm de `feathersheet-core::funcoes`.
A escolha de categoria restringe as opções. A aparência dos popups e dos
seletores básicos é compartilhada via `feather-ui::componentes` e
`feather-ui::tema`, reutilizável no Docs e no Slide. A inclusão no GitHub
não substitui `cargo check`/testes manuais no sistema operacional.

**Estado da suíte:** FeatherSlide já é GUI nativa (legado TypeScript removido);
FeatherDocs possui GUI Iced utilizável, com Tauri legado ainda presente;
FeatherSheet possui GUI Iced em migração, com Tauri legado mantido;
FeatherCAD ainda não possui crate `feathercad-gui` Iced e mantém Tauri;
FeatherPDF não possui editor GUI. Não usar percentuais antigos dos produtos
como medida de migração da interface.

## Incremento — Ribbon/Iced reestruturada a partir do TypeScript (08/10/2026)

**Implementação integrada à `main` após o merge do PR #10 em 08/10/2026.** O código antigo de `App::view_faixa` foi substituído por `crates/feathersheet-gui/src/app/faixa_nativa.rs`, mantendo as ações nativas em `app.rs` e reutilizando `feather_ui::componentes::{botao_grande,botao_pequeno,grupo_faixa}`. Os estilos são sincronizados pela célula ativa.

- **Arquivo:** Documento, Salvar e Exportar.
- **Página Inicial:** Área de transferência, Fonte, Alinhamento, Número, Estilos e Edição.
- **Inserir:** Tabelas, Ilustrações, Células, Fórmulas e Links.
- **Dados:** Classificar e filtrar, Ferramentas de dados e Consultas.
- **Exibir:** Mostrar, Janela, Zoom e Impressão.
- **Macros:** Editor, Executar, Planilha e Ajuda.
- **Ferramentas:** Automação, Aba, Proteção e Aplicativo.

O seletor de formatos reflete `Style.format`; fonte e tamanho usam `fontFamily` e `fontSize`; botões refletem negrito/itálico/sublinhado/tachado, alinhamento horizontal/vertical, moeda e porcentagem. Os menus suspensos são `pick_list` do Iced; ações nativas cobrem estilo de texto e preenchimento, bordas, famílias e tamanhos de fonte, casas decimais, limpar formatação, estimativa de largura da coluna, inserção/exclusão de linhas/colunas, classificação de intervalo, função, tema e zoom. O catálogo de funções tem opção `Todas` baseada na tabela de ajuda do motor Rust (161 verbetes) e categorias do motor.

Busca no cabeçalho executa somente comandos existentes em Rust. O cabeçalho da faixa pode ser rolado horizontalmente e os grupos são acessíveis por rolagem horizontal sem descarte silencioso. A exibição da barra de fórmulas pode ser alternada fora do modo de edição, e seu estado não é aplicado durante uma edição em andamento. F1, F12 e Ctrl+F1 permanecem.

**Limites:** a estrutura visual e as categorias foram recuperadas, mas menus sem equivalentes nativos (impressão, tabelas, gráficos, OpenQuery, formatação condicional, proteção, congelar) estão presentes desativados, não são implementações fictícias. O conjunto das 161 funções é o catálogo do motor, não prova de suporte a integrações externas em todas elas. A renderização de fontes/bordas/alinhamentos foi ampliada; quebra de linha real, mesclagem, pintor de formatação, filtros e menus contextuais avançados ainda precisam de trabalho e testes visuais. É obrigatório aguardar `cargo check` e `cargo test` da última revisão do PR antes de integrar à `main`.

## Incremento — restauração dos menus TypeScript no Rust/Iced (08/10/2026)

**Histórico de integração:** a branch `feat/feathersheet-native-menus` foi integrada à `main` via [PR #10](https://github.com/M4R4G0N/FeatherOffice/pull/10), mesclado em 08/10/2026. O frontend legado ainda serve como referência.

**Referência de origem verificada:** `src/ui/barras/menus.ts`, `ribbon.ts`, `toolbar.ts` e `src/ui/app/botao-direito.ts`. A lista original de **Arquivo, Editar, Ver, Inserir, Formatar, Dados, Macros, Ferramentas e Ajuda** estava preservada em `crates/feathersheet-gui/dados/menus.json`; o problema na interface Iced era o aplainamento dos submenus e despacho por rótulo.

| Superfície | Implementado nesta branch | Ainda não há paridade |
| --- | --- | --- |
| Menu principal | Catálogo tipado `app/menu_nativo.rs` com submenus recursivos, categorias na ordem TypeScript, separadores, atalhos, ícones da suíte, navegação inicial pelo teclado, Esc e clique fora | Hover para abrir submenu, estado visual individual para todos os estilos, menus de recentes e todos os diálogos antigos |
| Comandos nativos | Identificador + argumento original, operações conectadas a `Workbook`: abrir, salvar, XLSX, edição de seleção, inserir linhas/colunas/abas, formatação de texto/número/alinhamento, zoom, tema, ordenação de intervalo, remover duplicados, busca, macros, monitor, ajuda e configurações | Funcionalidades sem suporte nativo (ex.: tabela dinâmica, impressão, OpenQuery, filtros, congelamento, gráficos, condicional): folhas ficam desabilitadas, não executam falso sucesso |
| Ribbon | Recuperada aba **Arquivo**, com grupos Documento/Salvar/Exportar/Histórico; Página Inicial e outras abas ampliadas; parte dos botões passa a reutilizar `feather_ui::componentes::botao_pequeno` | Grupos TypeScript restantes, seletores complexos, botões grandes, layouts responsivos e abas de contexto |
| Menu de contexto | Botão direito no Canvas diferencia célula, cabeçalho de linha e coluna; abas expõem menu próprio; cópia, colagem, edição, inserção/exclusão e renomeação usam comandos existentes | Menus de objetos/tabelas, várias ações analíticas, múltiplas linhas/colunas por operação e acabamento de posicionamento responsivo |
| Regressão | Testes de integridade do JSON original e mapeamento de comandos; workflow de `pull_request` habilitado | Testes manuais de mouse, teclado, menu sobre a grade e aparência em Linux/Windows |

**Regra da migração:** não tratar funções ausentes como entregues; os itens sem implementação equivalente ficam desabilitados ou ausentes até o motor Rust disponibilizar a operação. F1, F12, Configurações, temas, cromo global e fluxo de fórmulas permanecem fora da refatoração estrutural.

## Progresso da implementação — etapa de seleção/teclado (08/10/2026)

**Código integrado na `main`. Validação automatizada: [GitHub Actions #37807897021](https://github.com/M4R4G0N/FeatherOffice/actions/runs/37807897021), compilação e 19 testes Rust/Iced aprovados. Ainda sujeito a teste visual e a critérios de aceite completos.**

| IDs relacionados | Entregue em Rust/Iced | Pendente para concluir |
| --- | --- | --- |
| FSM-0002/0003 | `selecao.rs` com estado de âncora/intervalo separado, `EstadoPonteiro` no Canvas, modo de edição da barra fx | Separar mais responsabilidades de `app.rs`; testes de foco real com IME |
| FSM-0101/0102 | `Shift+clique`, seleção por arrasto, desenho do intervalo, pressão/soltura e proteção contra arrasto preso | Auto-rolagem quando o ponteiro ultrapassa os limites do viewport |
| FSM-0103 | Clique nos cabeçalhos e canto seleciona linhas, colunas e toda a pasta | Conferir visualmente em rolagem extrema, zoom e panos congelados |
| FSM-0104/0105 | Setas, Shift, Ctrl+setas via `borda_dos_dados`, Home/End/PageUp/Down/Enter/Tab | Validar foco e modificadores no Linux real; testes de `Ctrl+Shift` |
| FSM-0106/0108 | F2, clique e duplo clique passam a ativar modo de edição da barra fx | Editor *inline* sobre a célula e seleção correta de texto |
| FSM-0109/0110 | Seleção representada por dois extremos O(1); hit-tests, mouse e teclado em testes unitários | Testes de janela interativa, desempenho e arrasto com auto-rolagem |
| FSM-0200 parcial | Copiar/recortar intervalo TSV e colar via Rust com transação de undo, limite 100 mil células | Rich clipboard, estilos, referências deslocadas, alça de preenchimento |

**Nota:** os testes de compilação/integração automatizados estão registrados no
[workflow Rust/Iced](https://github.com/M4R4G0N/FeatherOffice/actions/workflows/feathersheet-rust.yml).
Não marcar como concluídas as fases acima somente porque o código compila:
seleção, foco e arrasto dependem de verificação visual com mouse e teclado.

## Incremento — arrasto pela borda e redimensionamento (08/10/2026)

Implementação inicial integrada à grade Iced (a validar no CI): arrastar a borda externa
da seleção para mover o intervalo usando `Workbook::mover_intervalo` (fórmulas,
estilos, referências e histórico do motor); arrastar divisórias nos cabeçalhos para
alterar largura/altura com guia visual em pixels. Dimensões agora geram passos no
histórico de `Sheet` para Ctrl+Z/Ctrl+Y. Incluídos testes unitários de gestos,
mapeamento de divisórias, movimento e desfazer/refazer.

A alça de preenchimento (canto inferior direito da seleção) também foi ligada
ao `Sheet::preencher`: o motor aplica séries e fórmulas em uma transação,
com proteção de 100 mil células por gesto e novos testes unitários.

**Ainda pendente**: teste manual do gesto no Linux/Windows, autoajuste por duplo
clique, cópia por Ctrl+arrasto, alça de preenchimento, auto-scroll e orientação visual
do intervalo movido durante o arrasto. FSM-0201/0203 seguem parciais até esses aceites.

## Incremento — cópia com Ctrl+arrasto e autoajuste (08/10/2026)

O motor agora copia intervalos com sobreposição segura e fórmulas de referências
relativas deslocadas, mantendo estilos e histórico. O mouse distingue mover
(arrasto da borda) de copiar (Ctrl+arrasto da borda). Duplo clique no separador
de cabeçalhos aciona autoajuste de dimensão e Ctrl+Z desfaz o ajuste.

O autoajuste é uma **estimativa limitada** (até 10 mil linhas iniciais + 256
finais por coluna, até 256 colunas por linha), não medição tipográfica do Iced.
Permanece pendente o teste visual e o autoajuste exato em arquivos grandes.
FSM-0201/0203 continuam parciais até validação manual.

## Incremento — auto-rolagem de seleção (08/10/2026)

Seleção por arrasto tem detecção de bordas e pulso Iced enquanto o botão
esquerdo fica pressionado. A grade rola em intervalos de 90 ms e expande
o intervalo selecionado sem alocar células individualmente; ao soltar,
cancelar ou sair da grade a assinatura do pulso é desativada.

A auto-rolagem de mover/copiar intervalos e da alça de preenchimento
ainda não usa esse pulso. Os testes manuais em Linux e Windows continuam
necessários antes de marcar FSM-0102/FSM-0207 como concluídos.

## Incremento — edição de fórmulas com prévia nativa (08/10/2026)

Código adicionado à GUI Iced:

- `edicao_formula.rs`: estado separado para rascunho, autocomplete e
  referências; usa `suggest_at`, `active_call` e `accepts_reference`
  do motor Rust. Inclui testes de sugestões e referências UTF-8.
- Barra `fx` apresenta sugestões e assinaturas enquanto se digita; Enter
  ou clique aceita a sugestão. Durante digitação, a célula mostra
  imediatamente o rascunho sem gravá-lo em `Workbook`.
- Clique/arrasto na grade, quando a fórmula aceita referência, escolhe
  célula ou intervalo sem trocar a célula que está sendo editada;
  setas/Shift+setas e F2 atuam no modo de apontamento.
- O Canvas destaca os intervalos referenciados com cores.
- Esc cancela o rascunho, enquanto Enter/✓ confirma a fórmula no motor.

**Limitações ainda abertas**: o editor `text_input` atual acompanha a
inserção de referências no final do texto (não no cursor quando este é
reposicionado no meio), faltam editor inline, testes visuais com teclado,
composição IME e integração da referência entre abas. Não considerar
FSM-0401/0402/0403 encerradas antes desses aceites.

## Incremento — cursor e referências entre abas (08/10/2026)

- A fonte da edição é guardada em `EditorFormula::origem`, independente da aba
  que está aberta para escolher células. Trocar de aba não confirma o rascunho.
- Clique e arrasto na aba de destino montam referências como
  `'Base de Vendas'!C2:E4`; Enter confirma na aba de origem e Esc
  cancela sem gravar, retornando à célula original.
- Referências coloridas da fórmula são renderizadas na aba correspondente,
  não sobre células homônimas de outras abas.
- `edicao_formula.rs` passou a acompanhar índices de caracteres e preservar
  texto depois do cursor ao completar funções e inserir intervalos.
- `campo_fx.rs` delega o desenho/eventos ao `TextInput` Iced 0.13 e
  comunica o caret nativo do campo ao clicar ou selecionar texto nele.
- Ctrl+setas navega por blocos de dados enquanto aponta referências;
  o foco retorna ao índice de inserção, sem enviar o caret para o fim.

Teste de compilação e do fluxo entre abas permanece no GitHub Actions;
validação de mouse e IME em Linux/Windows continua pendente.

## 1. Diagnóstico verificado em 08/10/2026

A interface legada reúne funcionalidades nos seguintes módulos:

| Subsistema | Referência do frontend antigo | Situação na nova GUI Iced |
| --- | --- | --- |
| Grade | `src/ui/grade/grid.ts`, `desenho*.ts`, `vista.ts` | Canvas inicial já desenha viewport e estilos básicos; interação limitada |
| Mouse e arrasto | `grade/mouse.ts`, `arrastar.ts`, `eventos.ts`, `preencher.ts` | Só clique de célula e rolagem; falta seleção/drag-resize/preenchimento |
| Teclado | `grade/teclado.ts`, `app/teclas-globais.ts`, `grade/edicao.ts` | Tratamento global básico; sem máquina de estados/foco por modo |
| Menu e faixa | `barras/ribbon.ts`, `menus.ts`, `toolbar.ts`, `app/botao-direito.ts` | Barra global, abas e ações parciais; falta hierarquia, contexto e paridade |
| Fórmulas | `grade/f4.ts`, `grade/edicao.ts`, `barras/function-menu.ts` | Cálculo via motor; editor de fórmulas incompleto |
| Janelas | `src/ui/paineis/`, `src/ui/caixas/` | Configurações, Ajuda e Monitor independentes; demais diálogos ausentes |
| Dados/arquivo | `src/ui/app/`, `src/io/` | FSHZ, XLSX, CSV/TSV básicos; faltam operações e opções |
| Recursos avançados | `src/ui/{grafico,dinamica,condicional,protecao}/` | Motor Rust com várias operações; GUIs ainda ausentes |

**Dois defeitos de arquitetura a corrigir antes de continuar o visual:**

1. `grade.rs` captura botão esquerdo **pressionado** e roda do mouse, mas não captura `CursorMoved` e `ButtonReleased`. A seleção por arrasto não pode funcionar enquanto isso continuar assim.
2. `App::subscription` trata atalhos em eventos `Status::Ignored`; `text_input` e outros widgets podem capturar eventos. Não há roteamento de teclado por contexto de foco, modo de edição ou composição de texto. Isso não equivale à navegação da planilha antiga.

**O que já existe em Rust não deve ser duplicado:** avaliação, fórmulas, dados esparsos, estilos, histórico, IO FSHZ/XLSX, filtros, operações de linhas/colunas, mecanismos de validação e parte das ferramentas analíticas. Verificar API e testes antes de criar novas estruturas.

## 2. Regras técnicas obrigatórias

- **100% Rust + Iced no executável final:** sem Node, npm, TypeScript, WebView ou Tauri no fluxo de execução. O antigo frontend permanece somente como referência e recuperação funcional até a paridade.
- **`feather-ui` é o template global:** título, minimizar/maximizar/fechar, ajuda, monitor, configurações, componentes e atalhos reutilizáveis; comportamento específico da grade fica no FeatherSheet.
- **Separar eventos, estado e desenho:** não aumentar `src/app.rs` indefinidamente. `Canvas` não calcula fórmulas nem grava arquivo.
- **Estado explícito de interação:** `Normal`, `Selecionando`, `EditandoValor`, `EditandoFormula`, `ArrastandoCelulas`, `Preenchendo`, `Redimensionando`, `Menu` e `Modal`. O mesmo evento não pode editar texto e navegar simultaneamente.
- **Dados e estilos em Rust:** nenhuma segunda representação independente de `Workbook`. Operações mutáveis passam pelos métodos transacionais do motor, com desfazer/refazer correto.
- **Caminho quente rápido:** consultar valores visíveis em lotes com cache e versão; não percorrer toda a aba, nem fazer milhões de `get_value` em `draw`. Respeitar a arquitetura descrita em `docs/arquitetura/ARQUITETURA.md`.
- **Compatibilidade primeiro:** abrir/salvar preserva conteúdo, fórmulas, nomes, estilos, gráficos, tabelas, macros e metadados **mesmo que a GUI ainda não consiga editá-los**. Nunca apagar silenciosamente o que não for suportado.
- **Acessibilidade e sistema:** teclado completo, foco visível, zoom, alto DPI, janela redimensionável, tema; Linux e Windows; textos e atalhos em PT-BR.
- **Sem botões falsos:** cada comando exibido funciona ou fica claramente indisponível com explicação. Testes automáticos e verificação visual antes de considerar um item concluído.

## 3. Organização prevista (sugerida, sujeita a revisão da API Iced)

```text
feathersheet/crates/feathersheet-gui/src/
  main.rs                   # daemon Iced e inicialização
  app.rs                    # orquestração, roteamento e composição
  estado/
    mod.rs                  # estado principal e documento
    selecao.rs              # intervalo ativo, âncora e extensões
    edicao.rs               # estados de edição/fórmula e foco
    interacao.rs            # máquina de estados do ponteiro e teclado
    viewport.rs             # offset, zoom, congelamento e medidas
  grade/
    mod.rs
    desenho.rs              # canvas do viewport, estilo e sobreposições
    entrada_mouse.rs        # botão, arrastar, soltar, duplo clique, scroll
    entrada_teclado.rs      # navegação em modo Normal
    hit_test.rs             # célula/cabeçalho/borda/alça/aba
    preenchimento.rs       # handle, mover, copiar, série
    cache.rs                # blocos visíveis/versão
  interface/
    faixa.rs                # abas e grupos de comandos do Sheet
    menus.rs                # estrutura e submenus, contexto, estado
    formulario.rs           # barra de fórmulas, caixa nome
    abas.rs                 # tabbar
    status.rs               # cálculos da seleção/progresso
  comandos/
    mod.rs                  # despacho tipado; sem comando fake
    edicao.rs
    arquivo.rs
    estrutura.rs
    formato.rs
    dados.rs
  janelas/
    configuracoes.rs
    buscar.rs
    dialogos.rs
    painel_funcoes.rs
    ...
```

As divisões são por responsabilidade. Evitar espelhar literalmente todos os arquivos TS: **migrar comportamento, não sintaxe**. Subcomponentes generalizáveis vão para `feather-ui` e ficam disponíveis ao Slide/Docs/CAD.

## 4. Fases em ordem de execução

### Fase 0 — Fundamentos e estabilidade (P0)

- [ ] **FSM-0001** — Resolver warnings e garantir `cargo fmt --check`, `cargo clippy -D warnings`, `cargo check` e `cargo test` para o crate GUI.
- [ ] **FSM-0002** — Isolar `App`, `Msg` e componentes: reduzir concentração em `app.rs` por mudanças incrementais que compilam.
- [ ] **FSM-0003** — Máquina de estados de foco/edição/seleção; atalhos nunca roubam tecla do editor nem de janela auxiliar.
- [ ] **FSM-0004** — Testar persistência de `Workbook` com ficheiro FSHZ contendo fórmulas, estilos, dimensões, abas e objetos avançados.
- [ ] **FSM-0005** — CI Rust/Iced independente do workflow legado; publicar resultados de `check`, `test`, `clippy`, formatação e builds Linux/Windows.
- [ ] **FSM-0006** — Preparar roteiros manuais de aceitação e capturas de tela comparando legado/Iced.

**Aceite:** versão sem falhas de compilação, sem perda silenciosa de dados, e com testes de seleção/edição isolados antes de ampliar ferramentas.

### Fase 1 — Seleção e teclado iguais aos de planilhas (P0, primeiro trabalho funcional)

- [ ] **FSM-0101** — Seleção simples e por `Shift+clique`, com âncora, célula ativa e intervalo retangular desenhado.
- [ ] **FSM-0102** — Seleção por clique, arrastar e soltar, incluindo arraste fora do viewport com auto-scroll.
- [ ] **FSM-0103** — Clique em cabeçalhos seleciona linha/coluna; canto superior esquerdo seleciona a planilha.
- [ ] **FSM-0104** — Setas e `Shift+seta` movem/estendem; `Ctrl+seta` pula bloco; `Ctrl+Shift+seta` estende até borda.
- [ ] **FSM-0105** — `Home`, `Ctrl+Home`, `End`, `Ctrl+End`, `PageUp/Down`, `Tab`, `Shift+Tab`, `Enter`, `Shift+Enter`.
- [ ] **FSM-0106** — `F2` edita; digitar substitui valor; `Esc` cancela; `Enter` confirma e move; clique fora segue regra clara de commit.
- [ ] **FSM-0107** — Foco correto na grade após clique ou confirmação; atalhos globais não interferem em `text_input`, diálogos, nome de aba ou edição de fórmula.
- [ ] **FSM-0108** — Duplo clique começa a editar (não mover a planilha); cliques isolados não disparam ação duplicada.
- [ ] **FSM-0109** — Seleções grandes sem alocar uma estrutura por célula; blocos e limites seguros.
- [ ] **FSM-0110** — Testes automatizados de hit-test, arrasto, modifiers, foco e limites; testes manuais com mouse e teclado reais.

**Aceite:** selecionar `A1:D20` arrastando, `Shift+seta` estender seleção, `Ctrl+seta` pular dados, F2 editar, Esc cancelar, Tab/Enter mudar célula **sem travar nem alterar a célula errada**.

### Fase 2 — Arrastar, mover, preenchimento e viewport (P0/P1)

- [ ] **FSM-0201** — Pegar seleção pela borda e **mover ou copiar** intervalo mantendo fórmulas, estilos e referências via operação do motor.
- [ ] **FSM-0202** — Alça de preenchimento arrastável, sequência, datas/textos numerados, fórmulas com referências relativas/absolutas; duplo clique preenche até borda.
- [ ] **FSM-0203** — Redimensionar largura/altura pelo cabeçalho com cursor apropriado, indicador de tamanho e duplo clique para autoajuste.
- [ ] **FSM-0204** — Rolagem vertical/horizontal suave, barra de rolagem navegável, `Shift+roda` horizontal e zoom por `Ctrl+roda`.
- [ ] **FSM-0205** — Congelar linhas/colunas; selecionar/mover com pane congelado; `scrollIntoView` correta.
- [ ] **FSM-0206** — Arrastar separadores e reordenar abas; rolador de abas; ocultar/mostrar aba se suportado pelo formato.
- [ ] **FSM-0207** — Estado de ponteiro com arraste iniciado/movendo/solto/cancelado, inclusive sair da janela e perder foco.
- [ ] **FSM-0208** — Testar mouse em bordas, zoom, limites grandes e linhas/colunas de tamanhos variados.

**Aceite:** arrastar `A1:B4` para `D1:E4`, puxar a alça, redimensionar uma coluna e congelar cabeçalho sem corromper dados; desfazer/refazer restaura tudo.

### Fase 3 — Barra, menus e template FeatherOffice (P0/P1)

- [ ] **FSM-0301** — Migrar o **comportamento** de `barras/ribbon.ts`, `menus.ts`, `toolbar.ts` e `app/botao-direito.ts` para componentes Iced.
- [ ] **FSM-0302** — **Menu global** no `feather-ui`: popup ancorado, submenus, separadores, ícones, check/radio, disabled, tooltips, atalhos, hover e fechamento por Esc/clique fora.
- [ ] **FSM-0303** — Faixa com agrupamento de ações, ícones, prioridades responsivas, recolher/expandir (Ctrl+F1), seleção ativa e busca de comandos.
- [ ] **FSM-0304** — Menus **Arquivo / Início / Inserir / Layout/Exibir / Fórmulas / Dados / Ferramentas / Ajuda** com correspondência verificada no inventário antigo.
- [ ] **FSM-0305** — Context menu distinto para célula, faixa selecionada, cabeçalho linha/coluna, aba e objetos.
- [ ] **FSM-0306** — Toolbar: formatação, zoom, desfazer/refazer, salvar, impressão e atalhos configuráveis.
- [ ] **FSM-0307** — Barra personalizada/F1/F12/Configurações permanecem geridas por `feather-ui` sem duplicações; janelas auxiliares independentes.
- [ ] **FSM-0308** — Barra de status: soma, média, quantidade e progresso calculados pelo motor; menu de contexto na seleção.
- [ ] **FSM-0309** — Testes de despacho menu → comando → motor → redesenho; nenhum botão aparente sem ação real.

**Aceite:** a estrutura de menus é tão navegável quanto a antiga, todos os atalhos mostram rótulos corretos, botão direito abre operações específicas e a janela de Configurações não reduz a grade.

### Fase 4 — Editor de fórmulas e formatação avançada (P1)

- [ ] **FSM-0401** — Editor dentro da célula e barra `fx` sincronizados, cursor e seleção de texto, commit/cancel.
- [ ] **FSM-0402** — Referência por clique e arraste durante fórmula, realce de intervalos por cor, tecla F4 e referências absolutas/relativas.
- [ ] **FSM-0403** — Autocomplete de funções, assinaturas, sugestões, documentação e separadores PT-BR.
- [ ] **FSM-0404** — Fontes, tamanhos, cor de texto/fundo, bordas, numeração, casas decimais, wrap, rotação e alinhamento.
- [ ] **FSM-0405** — Mesclar/desmesclar células, fórmulas derramadas, seleção visual de intervalos mesclados.
- [ ] **FSM-0406** — Renderizar formatação condicional e validações; conflitos de estilos e células vazias.
- [ ] **FSM-0407** — Colar especial (valores, fórmulas, estilos, transpor), colagem multibloco e formatação na mesma transação.
- [ ] **FSM-0408** — Compatibilidade pt-BR e testes: datas, moeda, porcentagem, erros, valores grandes, `SOMA` etc.

**Aceite:** digitar `=SOMA(`, selecionar `A1:A10` com mouse, concluir fórmula, copiar com referência deslocada e renderizar os estilos existentes no arquivo.

### Fase 5 — Operações da planilha e arquivos (P1)

- [ ] **FSM-0501** — Operações de área (inserir/excluir/deslocar, limpar conteúdo/estilo, linhas/colunas e abas com histórico).
- [ ] **FSM-0502** — Buscar/substituir em uma ou todas as abas, navegar resultados e realçar ocorrências.
- [ ] **FSM-0503** — Classificar, filtros, tabelas, deduplicação, dividir texto, intervalos nomeados e validação.
- [ ] **FSM-0504** — Janelas de abrir/salvar/exportar com recentes, filtros por extensão, cancelamento, erros e operação em background.
- [ ] **FSM-0505** — Importar/exportar `.fshz`, `.xlsx`, CSV/TSV e demais formatos legados relevantes sem truncamento silencioso.
- [ ] **FSM-0506** — Impressão/PDF, área de impressão, prévia, quebras e cabeçalho/rodapé.
- [ ] **FSM-0507** — Recuperação/autosalvamento, crash recovery, arquivo temporário atômico, backups e abertura protegida de arquivos grandes.
- [ ] **FSM-0508** — Testes comparativos com arquivos reais, casos com dados grandes e reabertura em ambos os frontends.

**Aceite:** abrir arquivo complexo do legado, editar e salvar; reabrir sem perder fórmulas, estilos, objetos, nomes ou metadados não editados.

### Fase 6 — Componentes analíticos e painéis (P2)

- [ ] **FSM-0601** — Gráficos em Rust/Iced: seleção de dados, criação/edição, mover e redimensionar no canvas, exportar/imprimir.
- [ ] **FSM-0602** — Tabelas dinâmicas: editor de campos, medidas, filtros, atualização e formatos.
- [ ] **FSM-0603** — Formatação condicional: regras, editor, prévia e desenho no viewport.
- [ ] **FSM-0604** — Validação, listas suspensas, proteção de célula/intervalo e mensagens.
- [ ] **FSM-0605** — OpenQuery: navegador de fontes, etapas, transformações, dependências, atualização e progresso **sem interface web**.
- [ ] **FSM-0606** — Ferramentas de análise, estatísticas, diagnósticos, perfil de colunas e funções.

**Aceite:** cada ferramenta reproduz operações que já existem no motor e recupera seu estado ao reabrir o arquivo.

### Fase 7 — Macros e automação, sem dependência de JavaScript (P2)

- [ ] **FSM-0701** — Inventariar macros antigas e metadados executáveis; distinguir código legado de comportamento do motor.
- [ ] **FSM-0702** — Definir runtime seguro e local para macros Rust (por exemplo, engine interpretada com permissões/limites), **sem exigir Node ou WebView**.
- [ ] **FSM-0703** — Editor de macros Iced, parâmetros, atalhos, botões, eventos e logs; integração transacional ao motor.
- [ ] **FSM-0704** — Política de compatibilidade: preservar scripts JS antigos como dados, sem executar silenciosamente; oferecer migração/exportação/documentação por macro.
- [ ] **FSM-0705** — Testes de sandbox, recursos, timeout, acesso a arquivo/rede e cancelamento.

**Aceite:** nenhuma macro antiga é perdida silenciosamente; as suportadas executam de modo seguro sem frontend JS.

### Fase 8 — Desempenho, UX, multiplataforma e aposentadoria do legado (P1/P2)

- [ ] **FSM-0801** — Caching de viewport/versionamento, redraw parcial, medidas de FPS e latency; operações grandes em worker Rust sem travar UI.
- [ ] **FSM-0802** — Testes com 100 mil, 1 milhão e 2 milhões de linhas e planilhas com muitas fórmulas; registrar RAM, tempo de scroll/edição/abertura/salvamento.
- [ ] **FSM-0803** — Acessibilidade, múltiplos monitores, Wayland/X11, DPI, janelas próprias, fontes e atalhos.
- [ ] **FSM-0804** — Builds executáveis Linux/Windows com CI, instalador/pacote, ícones e atualização.
- [ ] **FSM-0805** — Matriz de paridade: cada item relevante do `docs/planejamento/ROADMAP.md` tem status **motor Rust**, **GUI Iced**, **teste**, **observações** separados.
- [ ] **FSM-0806** — Somente com paridade aprovada: remover Tauri/TypeScript/Node/WebView do produto e dos workflows; manter o histórico no Git, não apagar antes.

**Aceite final:** FeatherSheet abre como aplicativo nativo Rust/Iced, preserva arquivos legados, suporta fluxos de teclado/mouse, usa o visual global FeatherOffice e não precisa de runtime web nem para executar, nem para construir a versão final.

## 5. Sequência de entregas recomendada (primeiras sessões)

Cada entrega vai **diretamente para a `main`**, com CI verde antes da próxima.

1. **Entrega A: seleção e teclado.** FSM-0002/0003, 0101 a 0110. Prioridade absoluta: corrigir setas, edição/foco, `Shift` e seleção arrastada.
2. **Entrega B: arrasto e viewport.** FSM-0201 a 0208. Prioridade: mover/copiar bloco, alça de preenchimento, resize, rolagem e congelamento.
3. **Entrega C: menus e Ribbon.** FSM-0301 a 0309. Migrar comportamento e layout real do antigo, aproveitando `feather-ui`, **sem simplificar as opções que já funcionavam**.
4. **Entrega D: fórmulas e estilos.** FSM-0401 a 0408. Completar referência interativa, edição inline, numeração e cores.
5. **Entrega E: dados e arquivos.** FSM-0501 a 0508. Depois recursos avançados Fases 6–8.

## 6. Critérios para marcar uma tarefa [x]

Uma tarefa só está **Feita** quando cumpre **tudo**:

1. Funciona no Rust/Iced, sem chamadas a TS/Tauri.
2. Possui teste automatizado para estado/evento/motor e roteiro manual de interação para UI.
3. Compila com `cargo check`; passa em `cargo test`, `cargo fmt --check` e `clippy` (ou ressalva documentada).
4. Não regressa edição, undo/redo, fórmulas, formato nem troca de abas.
5. Não cria ação visual vazia e usa os componentes `feather-ui` onde cabível.
6. O documento foi atualizado com commit e evidência do teste.

**Não presumir paridade porque um menu com o mesmo nome existe.** Migrar significa reproduzir comportamento, navegação, acessibilidade, formatos e estado. O legado é **referência de aceite** até a última fase.
