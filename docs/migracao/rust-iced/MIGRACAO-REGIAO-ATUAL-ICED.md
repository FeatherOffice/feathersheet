# Selecionar Região Atual — integração nativa TypeScript → Rust/Iced

**Origem TypeScript:** `feathersheet/src/core/ops.ts::regiaoAtual`.
Trecho preservado integralmente em `descarte/feathersheet/typescript/core/regiaoAtual.ts.legacy.txt`.

O algoritmo já existia no motor `feathersheet-core/src/operacoes/ops.rs::regiao_atual`
(é utilizado pelo assistente **Remover duplicados**). Esta rodada
conecta-o à interface Iced em `feathersheet-gui/src/app/selecao_regiao.rs`,
sem duplicação de cálculos.

Acesso: **Dados → Ferramentas de dados → Região atual**;
**Página Inicial → Classificar e filtrar → Selecionar região atual**;
**Ctrl+Shift+8 / Ctrl+***. O Ctrl+A permanece selecionando a planilha toda.

A operação seleciona blocos contíguos incluindo diagonais; não atravessa
linhas/colunas totalmente vazias; limita seleção de coluna inteira à área usada;
não salta para dados distantes quando o cursor está fora da área utilizada;
expande mesclas; não grava alterações, não aloca células na planilha
e não interfere com desfazer/refazer. Apenas a seleção/viewport são alterados.

Não foi removido `src/core/ops.ts`: ele contém outras funções ainda usadas
em compatibilidade/como referência, e a antiga UI inteira já foi para
`descarte/feathersheet/legacy-frontend/`. Nada em `descarte/` é importado.

Testes escritos, sem execução no GitHub nem GitHub Actions.
Para validar localmente:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml selecao_regiao -j 2
```
