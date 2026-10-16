# Estatísticas da seleção — migração TypeScript → Rust/Iced

Referências históricas já em `descarte/`:
- `descarte/feathersheet/legacy-frontend/src/ui/app/app-dialogs.ts`
  (`showColumnStats`: oito indicadores em diálogo compacto).
- `descarte/feathersheet/legacy-frontend/src/ui/app/acoes-do-menu.ts`
  (ação `columnStats`).
- `descarte/feathersheet/legacy-frontend/src/ui/barras/ribbon.ts`
  (Dados → Estatísticas).

### Implementação nativa

- `feathersheet-gui/src/app/estatisticas_ui.rs` cria um diálogo Iced próprio,
  separado do **Perfil da Coluna**.
- Reutiliza `feathersheet_core::operacoes::ops::column_stats_com`, sem segundo
  cálculo no frontend; considera todo o retângulo selecionado e limita a última
  linha à última linha utilizada.
- Exibe células preenchidas, números, vazias, distintos, soma, média, mínimo
  e máximo. Não altera células, configurações, histórico ou arquivo.
- O cálculo maior que 4.096 células usa `spawn_blocking` após capturar os
  valores. Limite explícito de 100.000 células por análise, para proteger
  responsividade/memória; seleções maiores são recusadas sem truncamento.
- Botão **Dados → Estatísticas** e ID TypeScript `columnStats` apontam para
  `Msg::EstatisticasAbrir`; **Dados → Perfil da coluna** continua independente.
- O painel fecha com **Esc**, clique fora ou botão Fechar; resultado assíncrono
  atrasado é descartado após fechar, abrir outra planilha ou criar documento.

### Verificação

Testes Rust escritos no módulo, **não executados** nesta edição.
Não executar workflows GitHub Actions para esta migração.

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml estatisticas -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```
