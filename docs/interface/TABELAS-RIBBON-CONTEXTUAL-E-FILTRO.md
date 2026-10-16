# FeatherSheet — Design da Tabela na Ribbon e filtro no cabeçalho

**08/10/2026 · Implementação Rust/Iced na main · Cargo e smoke visual pendentes.**

A interface TypeScript `src/ui/barras/ribbon-tabela.ts` define uma **aba contextual**
para estilo e propriedades; o popup de filtro pertence ao cabeçalho da
coluna. O painel lateral monolítico de 475 px misturava essas funções
e cobria a grade.

## Nova interação

**Design da Tabela:** aba temporária visível enquanto a célula selecionada
estiver dentro de uma tabela verdadeira. Mostra o nome/renomeação,
converter em intervalo, faixas por linha e coluna, bordas, linha de
totais, função de total e sete temas com amostras de cor do
`feather-ui::formatacao::botao_cor`. Ao sair da tabela, a aba desaparece.

**Filtro do cabeçalho:** clicar na seta da coluna abre um menu suspenso
ancorado na coordenada da grade, não um painel lateral. Contém
ordenação A→Z/Z→A, pesquisa, caixa de seleção para os valores,
Todos/Nenhum, Aplicar, Limpar e critérios personalizados. Rolagem
interna e fechamento por ×, clique fora ou Esc.

Os comandos seguem delegados a `app/tabelas/migracao.rs`: nenhum
filtro, tema, total ou serialização de tabela foi reimplementado.
O codec `.fshz` e os dados de células permanecem intactos.

## Filtro virtualizado e leitura progressiva — 13/10/2026

- O menu da coluna faz a primeira leitura limitada a **8.192 linhas**;
  não carrega mais todos os valores na abertura.
- **Carregar +8 mil linhas** lê mais um lote. **Carregar tudo** percorre os
  lotes progressivamente (16 ms de intervalo); **Parar** interrompe a leitura.
- A lista é virtualizada por índice de rolagem: somente **18 caixas**
  são construídas em cada quadro, independentemente do total de distintos.
  A busca considera todos os valores **já carregados**, não toda a coluna
  até o usuário solicitar a varredura completa.
- Selecionar tudo/Limpar atua nos resultados da busca, mesmo que estejam
  fora do viewport. O estado dos valores de filtros anteriores é preservado.
- **Seleção incompleta** usa `not_in` para não esconder valores desconhecidos;
  a confirmação sem alterações não apaga critérios antigos. A avaliação
  do filtro ao clicar OK ainda percorre as linhas no motor Rust.
- Testar em XLSX grande com coluna de muitos distintos, filtro existente,
  busca após rolagem profunda, interrupção e reabertura em outra coluna.
  A implementação está **pendente de cargo check e smoke visual**.

## COMO TESTAR NO MEU COMPUTADOR

```bash
git status --short
git pull --ff-only origin main
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml design_da_tabela_fica_na_ribbon_e_filtro_no_cabecalho -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml converter_via_ribbon_nao_remove_celulas -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml tabelas:: -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

Criar uma tabela em A1:B4, selecionar A1, conferir aba **Design da
Tabela**, trocar tema e opções. Clicar na seta de B1 e conferir o
dropdown junto ao cabeçalho. Filtrar um valor, ordenar, limpar e fechar
pelo Esc. Selecionar célula fora da tabela: a aba contextual deve
desaparecer. Converter em intervalo e verificar que todos os dados
continuam salvos. Reabrir o FSHZ.

**Limitação:** Cargo/Rust e a GUI não estão disponíveis neste ambiente;
a interface ainda precisa de validação real de rolagem, foco, DPI e zoom.
