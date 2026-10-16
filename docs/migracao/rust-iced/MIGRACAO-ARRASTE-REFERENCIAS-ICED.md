# FeatherSheet — arraste de referências de fórmulas na grade Iced

Origem TS: `descarte/feathersheet/legacy-frontend/src/ui/grade/`, com a
matemática já portada em `feathersheet-core/src/formula/ref_drag.rs`.
O código antigo de interface continua no descarte; nada foi copiado para
outro motor.

Migração feita em 12/10/2026:
- A grade preserva as caixas com posições no texto da fórmula e nomes das abas,
  em vez de manter só `Rect`.
- Ao pressionar a **borda colorida**, inicia o movimento. A **quina inferior
  direita** redimensiona a referência; o interior conserva a seleção normal.
- A prévia verde mostra o novo retângulo **apenas nas células visíveis**;
  milhões de linhas não são materializados.
- Ao soltar, a grade envia somente a referência escolhida e o gesto ao app.
  O app reutiliza `drag_rect` e `rewrite_box` do core para conservar
  cifrões, referências de outras abas, texto restante e UTF-8.
- O gesto atua **somente no rascunho**; salvar continua sendo feito por
  Enter/✓, sem editar células durante o movimento. Há testes do hit-test e
  integração do cálculo existente.

Limites a validar: cursor fora do canvas ao soltar, deslocamento com painel
congelado, foco do campo FX, IME e efeitos visuais Linux/Windows. Não houve
execução de Cargo nem Actions; não considerar a migração aceita sem smoke visual.

Verificação local:
```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml grade::referencias -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```
