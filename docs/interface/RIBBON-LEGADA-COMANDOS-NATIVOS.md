# FeatherSheet — comandos legados da Ribbon conectados ao Rust/Iced

Migração incremental da interface TypeScript arquivada em `descarte/`.

## Fontes legadas já no descarte

- `descarte/feathersheet/legacy-frontend/src/ui/barras/ribbon.ts`
  define os botões de **Renomear…**, **Estatísticas** e **Tela inteira**.
- `descarte/feathersheet/legacy-frontend/src/ui/app/acoes-do-menu.ts`
  define as ações `renameFile`, `columnStats` e `fullscreen`.
- `descarte/feathersheet/legacy-frontend/src/ui/app/app-dialogs.ts`
  define o diálogo compacto de estatísticas com oito indicadores.

Esses arquivos já estavam guardados em `descarte/` antes deste incremento,
portanto **não foram duplicados** nem reintroduzidos no aplicativo Rust.

## Ligações novas no aplicativo nativo

- **Arquivo → Renomear…** e **menu Arquivo → Renomear arquivo**:
  `Msg::RenomearArquivo`, já implementado em
  `app/documento_operacoes.rs`. Permite escolher o novo nome por diálogo,
  salva antes caso haja alterações e evita sobrescrever arquivo existente.
  Em planilhas ainda sem caminho, segue para **Salvar como**.
- **Exibir → Tela inteira / Sair tela inteira**:
  `Msg::TelaInteira` alterna `window::Mode::Fullscreen` e
  `window::Mode::Windowed`. O comando de teclado **F11** já utilizava
  essa mensagem; agora a Ribbon oferece o mesmo recurso.
- **Dados → Estatísticas**:
  agora abre o diálogo próprio `Msg::EstatisticasAbrir` em
  `app/estatisticas_ui.rs`, migrado na rodada seguinte.
  Reutiliza `feathersheet_core::operacoes::ops::column_stats_com` para
  contagem, números, vazios, distintos, soma, média, mínimo e máximo no
  **retângulo inteiro selecionado**, inclusive com várias colunas.
  **Dados → Perfil da coluna** continua separado, com frequência, quartis,
  histograma e outliers.

**Paridade do diálogo compacto:** comportamento principal migrado para Rust,
com limite explícito de 100 mil células, cálculo assíncrono acima de 4.096
células e descarte de resultados antigos. O diálogo TypeScript original já está
arquivado em `descarte/`; ver
`feathersheet/docs/interface/ESTATISTICAS-SELECAO-RUST.md`.
Ainda faltam compilação e validação visual locais.

## Validação pendente

Foram escritos testes unitários Rust para renderização das abas afetadas,
alternância de tela inteira e uso do perfil nativo. **Não foram executados
testes, Cargo ou GitHub Actions** durante esta alteração.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml testes_acoes_legadas -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```
