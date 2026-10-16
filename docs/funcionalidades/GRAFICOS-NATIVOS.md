# Gráficos na interface nativa

Inserir → Gráfico abre um **dropdown com miniaturas na janela principal**, abaixo da faixa de ferramentas. Clique em uma miniatura para inserir o gráfico sobre as células e abrir o painel **Formatar gráfico** na lateral direita. Não abre uma janela auxiliar. Clique fora, novamente no botão ou pressione Esc para fechar a galeria.

Uma seleção de várias células é respeitada. Ao partir de uma única célula preenchida, o motor `Sheet::regiao_da_tabela` detecta automaticamente a tabela ao redor, até uma linha/coluna vazia. A fonte continua vinculada à aba pelo ID. Até 200.000 células são aceitas na extração síncrona; intervalos maiores recebem uma mensagem explícita.

A galeria mantém seis famílias e 25 miniaturas do TypeScript. Sete tipos têm renderizador disponível: colunas, barras, linhas, áreas, degraus, pizza e dispersão. Os outros 18 permanecem desabilitados. Pizza permite escolher uma série e rejeita negativos/totais nulos. A adaptação e agregação continuam no motor/IO, sem recalcular totais com dados amostrados.

O painel lateral permite editar intervalo, cabeçalhos, categorias, orientação e soma de categorias iguais; título, subtítulo, unidade e títulos dos eixos; nome/cor das séries, fundo, tamanho do texto, largura/altura do gráfico; visibilidade de título, legenda, eixos e valores. As alterações aparecem na prévia sobre a grade. **Salvar alterações** aplica a configuração; fechar o painel descarta alterações de configuração ainda não aplicadas. A inserção inicial já adiciona o gráfico à aba. Clique em Editar para abrir o painel; arraste o corpo ou o cabeçalho para mover e use a alça ↘ para redimensionar. Excluir gráfico remove somente o gráfico selecionado.

As configurações são gravadas no bloco `g` existente do `.fshz` e reaparecem ao reabrir a planilha. `feathersheet-io::graficos_salvos` preserva as outras entradas e campos de estilos desconhecidos; JSON inválido produz erro sem sobrescrever o conteúdo. As opções específicas da interface são extensões `_featherNative` e `_featherPieSeries`; consumidores antigos ignoram essas extensões. O formato-base não muda. A posição e o tamanho seguem `ChartConfig.position`. A exportação SVG continua disponível.

## Colorpicker compartilhado no painel de gráficos (08/10/2026)

O editor lateral **Formatar gráfico** agora usa o componente Iced
`feather_ui::seletor_cor::painel`, o mesmo já presente na Ribbon do
FeatherSheet e no FeatherSlide. Antes, cor de série, primeira série e fundo
tinham somente `text_input` com o hexadecimal; não havia botão de paleta.

- Ao lado de **Cor da primeira série**, **Cor de fundo** e **cada Série**
  há uma amostra da cor com botão **Paleta**.
- Clicar na amostra expande o seletor completo **dentro da área rolável do
  editor lateral**, com saturação/brilho, matiz, transparência (alfa),
  canais RGBA, hex e cores pré-definidas. Nenhuma janela auxiliar.
- Somente um seletor fica aberto por vez; clicar outra Paleta troca o alvo;
  clicar a mesma novamente, **Fechar** ou **Esc** oculta o seletor sem fechar
  o editor de gráficos.
- A cor escolhida atualiza a prévia usando Charts; o hexadecimal pode
  continuar sendo digitado diretamente. O motor gráfico aceita
  `#RRGGBB` e `#RRGGBBAA`; na saída SVG o fundo semitransparente
  usa `fill-opacity` e as séries preservam o alfa do Drawing.
- A ação **Sem cor** mantém uma cor válida para as séries principais/
  fundo (restaura os padrões) e remove apenas o override da série.
- `_featherNative` e `style.seriesColors` mantêm as mesmas chaves;
  a persistência do documento `.fshz` e a edição das células não mudam.

**Testes Rust adicionados, ainda não executados neste ambiente:**

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml colorpicker_ -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml grafico_aceita_rgb_e_rgba -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Roteiro visual:** criar gráfico com pelo menos duas séries, abrir o
painel lateral, rolar até **Séries**, clicar **Paleta** e alterar matiz,
brilho, RGB/hex e transparência; testar também **Cor de fundo**, fechar
pelo Esc, clicar **Salvar alterações**, fechar e reabrir o gráfico. A
prévia e as cores salvas devem coincidir e não pode haver alteração nas
células de origem.

## Correção de ancoragem do dropdown Gráfico (08/10/2026)

**Problema visual reportado:** a galeria de miniaturas aparecia por cima da
Ribbon/abas, sem relação com o botão **Inserir → Gráfico ▾**. O overlay
usava coordenadas fixas `x=120` e `y=96` para toda abertura.

**Mudança:** o clique no ícone guarda `graficos.ancora_seletor_x` antes
da reconstrução do estado. O posicionamento, em pixels lógicos, fica
horizontalmente abaixo do botão que abriu a galeria e verticalmente **após
a faixa inteira** (cromo + barra de abas + Ribbon). A geometria
`geometria_overlay::galeria_graficos` limita a largura, altura e X ao
tamanho real da janela (inclusive após resize). Em janelas baixas, o
conteúdo mantém rolagem interna, sem cobrir as abas.

A galeria continua fechando pelo **×**, clique fora e **Esc**, com as 25
miniaturas existentes e os sete tipos habilitados. Esta mudança é
**exclusivamente da interface**: não modifica gráficos salvos,
dados, motor Charts nem formato `.fshz`.

**Testes adicionados (não executados neste ambiente):**

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml galeria_graficos -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml galeria_do_botao_grafico_nao_segue_o_mouse_apos_abrir -j 2
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Aceite visual:** clicar **Inserir → Gráfico ▾** em janela normal e
mínima (760 × 520). O canto superior da galeria deve ficar sob a faixa,
o painel deve acompanhar a posição horizontal do botão e não ultrapassar
a borda direita/inferior, com rolagem nas miniaturas em janela baixa.
Mover o mouse após abrir não deve deslocar o popup.

## Correção de interação — mover, redimensionar e margens (08/10/2026)

Corrigido o caso reportado com duas linhas sem cabeçalho (`a,1` e `b,2`):
a primeira linha com valor numérico **não é descartada como título**, então
ambas as categorias entram no gráfico. Cabeçalhos textuais (`Categoria,Valor`)
continuam sendo reconhecidos e a opção manual `Usar cabeçalhos` permanece.

O gráfico novo nasce em **480 × 300 px**, em vez de 640 × 360.
O motor compartilhado Charts limita a largura de uma única coluna a aproximadamente
112 unidades SVG e a centraliza na região dos eixos, evitando um retângulo enorme
colado às extremidades. A composição Iced mantém fundo opaco, moldura suave e
**12 px de margem** para a imagem não misturar as bordas da grade.

**Interação no gráfico salvo:**
- Segure e arraste no corpo do gráfico ou no ícone `⠿` do cabeçalho para mover.
- Segure e arraste a **alça `↘` no canto inferior direito** para alterar
  largura e altura (largura 240–1600 px; altura 200–1600 px).
- Clique no botão **Editar** para abrir o painel lateral. Os campos de tamanho
  agora leem a posição salva e não restauram 640 × 360 ao reabrir.
- As coordenadas `x/y/width/height` continuam gravadas no bloco `g` do
  formato `.fshz`, sem modificar o conteúdo das células ou outros metadados.

Os gráficos antigos **mantêm suas dimensões salvas** até o usuário ajustá-las.
Não existe ainda posicionamento relativo às células com compensação de
rolagem/zoom: continua sendo coordenada de viewport. A edição por alça não
foi integrada ao histórico de undo, portanto use salvar/reabrir para confirmar
o resultado antes de descartar um arquivo antigo.

**Testes adicionados, ainda não executados neste ambiente:**
- `graficos::testes::grafico_duas_linhas_sao_dados_nao_cabecalho`;
- `graficos::testes::alca_redimensiona_preserva_posicao_e_reabre_dimensoes_editadas`;
- `featheroffice-charts::testes::coluna_isolada_fica_centralizada_sem_encostar_nas_bordas`;
- `graficos_salvos::testes::redimensionamento_preserva_posicao_e_fshz_apos_reabertura`.

Execute na raiz do repositório:

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -p feathersheet-gui graficos -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml -p feathersheet-io graficos_salvos -j 2
cargo test --manifest-path featheroffice/Cargo.toml -p featheroffice-charts -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

O aceite visual de mover, redimensionar e soltar continua aberto (**CHT-015 `[/]`**).

## Limitações atuais

A posição do desenho na interface nativa é relativa à área visível da grade; o acompanhamento de rolagem/zoom por âncoras de célula e o histórico de desfazer das configurações de gráficos ainda não estão implementados. Fontes e algumas opções avançadas do legado não têm editor nativo. Os outros 18 tipos não ganham renderizador por aparecerem na galeria.

## Validação

Em 08/10/2026 passaram 73 testes da GUI, 102 do IO (2 ignorados), 45 de Charts e 6 doctests de Charts, com 2 jobs, incluindo seleção automática/explicita, galeria, inserção/edição, arraste, exclusão, pizza com várias séries, SVG com textos escapados, erros de dados/cor e roundtrip `.fshz` com metadados preservados. O aceite visual do dropdown/painel lateral final permanece pendente; CHT-015 continua `[/]`.

```bash
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -p feathersheet-gui -p feathersheet-io -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

## Incremento CHT-009 — Área e Linha em degraus (08/10/2026)

O `featheroffice-charts` agora desenha duas modalidades adicionais: `Tipo::Area` fecha o polígono de cada série na linha **zero** da escala numérica e preserva valores negativos; `Tipo::Degraus` percorre segmentos horizontais seguidos por transições verticais, respeitando a posição de cada categoria. A extração/agregação de `feathersheet-core` não mudou: `ChartType::Area` e `ChartType::StepLine` são adaptados em `feathersheet-io::grafico_compartilhado`. A prévia Iced, SVG e PDF passam pela mesma `Cena` vetorial.

As variantes empilhadas, percentuais, combinados e os eixos secundários **ainda não** foram migrados: seguem desabilitados na galeria. CHT-009 fica em `[/]` até a implementação completa e o aceite do mantenedor.

```bash
cargo run --manifest-path featheroffice/Cargo.toml -p featheroffice-charts --example migracao_area_degraus -j 2 -- area > /tmp/featheroffice_area.svg
cargo run --manifest-path featheroffice/Cargo.toml -p featheroffice-charts --example migracao_area_degraus -j 2 -- degraus > /tmp/featheroffice_degraus.svg
xdg-open /tmp/featheroffice_area.svg
xdg-open /tmp/featheroffice_degraus.svg
```

Teste de regressão: `cargo test --manifest-path featheroffice/Cargo.toml -p featheroffice-charts -j 2`, `cargo test --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml grafico_compartilhado -j 2`, e `cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml graficos -j 2`. Registro do autor de CHT-009: Cargo não foi executado na entrega original. A integração com o editor lateral foi revalidada nesta execução.
