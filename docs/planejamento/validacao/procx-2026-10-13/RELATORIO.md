# PROCX compacto — execução e ponto de retomada

**Estado: FSH-PERF-001 `[/]`, implementação candidata; aceite pendente.**
Passos 2–9 não iniciados. Trabalho encerrado num ponto de retomada após o
usuário permitir parar para preservar o contexto. Nenhum passo foi contado
como concluído, nem houve commit/push desta execução.

## Implementação

- Referências qualificadas e faixas externas entram na `FormulaTable`.
  A segunda ponta de `Produtos!A1:A9` herda a aba da primeira; faixas locais,
  nomes, voláteis e fórmulas matriciais conservam o caminho avulso.
- Dependências externas ficam nos padrões/colunas; não há um milhão de
  entradas no `Sheet.externas`. A invalidação propaga pelas colunas
  dependentes e pelo grafo avulso, preservando colunas independentes.
- Colunas invalidadas usam um sinal atômico; os slots são invalidados quando
  um novo resultado é consolidado. O sinal é respeitado em leitura normal,
  busca, iteração e gravação, evitando ressuscitar valores antigos.
- Consultar uma fórmula externa sem `Workbook` não grava o erro no cache.
- Edições estruturais reescrevem as referências qualificadas e ambas as
  pontas de uma faixa. Renomeação, circularidade e undo/redo têm regressões.
- A carga de fórmulas avulsas de FSHZ usa o preparador por padrão, evitando
  disparar invalidação de edição a cada PROCX importado de arquivos antigos.

## Medições da candidata

Linux, Cargo 1.98.1, release, oito workers, arquivos reais do usuário.
RAM em **MiB**, medida por `/proc/self/status`; não confundir com MB decimal.
Baseline do motor: `2398e7d2` (antes das alterações desta execução).
Arquivos originais foram apenas lidos; gravações foram cópias em `/tmp`.

**Estas medições antecedem o último ajuste de `store`; não são aceite da
versão final.** A abertura FSHZ registrada também antecede a adaptação final
de carga por `CelulaPreparada`. É necessário repetir a comparação final.

| Medida na Planilha Pesada | Antes | Candidata |
|---|---:|---:|
| RAM calculada no app Iced | 1.857,9 MiB | 565,4 MiB |
| RAM calculada/consolidada, bancada XLSX sem janela | 1.756,6 MiB | 455,9 MiB |
| Abrir XLSX | 18,05 s | 17,22 s |
| Abrir FSHZ original | 9,51 s | 4,62 s |
| Corrente de 1 milhão, XLSX sem resultados guardados | 10,42 s | 8,88 s |
| Resumo inteira, bancada XLSX | 8,62 s | 7,72 s |
| Resumo inteira, bancada FSHZ | 7,46 s | 6,58 s |
| Calcular todas as células, após corrente/Resumo (XLSX) | 9,51 s | 8,23 s |
| Salvar cópia FSHZ calculada | 20,01 s | 15,46 s |
| Pico até calcular tudo, bancada XLSX | 1.878,6 MiB | 935,6 MiB |
| Pico incluindo gravação FSHZ, bancada XLSX | 5.601,3 MiB | 4.144,5 MiB |
| CPU do desenho Canvas Iced, p95 durante rolagem | 0,427 ms | 0,355 ms |
| CPU do desenho Canvas Iced, máximo | 0,603 ms | 0,481 ms |
| Atualização de rolagem e cálculo visível Iced, p95 | 0,294 ms | 0,042 ms |

Na aba Dados, 1.000.000 de avulsas passaram para a coluna: 7.000.000 de
fórmulas compactas, nenhuma avulsa. Os resultados consolidados saíram do mapa
extra. Os totais da corrente e da Resumo coincidiram nas duas versões.

A bancada gráfica usou cópias temporárias das fontes, a mesma GUI e
`feather-ui` v0.2.0 em ambas as versões: 150 eventos automáticos de rolagem,
1280 × 800, dados previamente calculados/consolidados, sem auto-salvamento.
Mede CPU de atualização e desenho Canvas, **não GPU/compositor nem custo
total de apresentação do quadro**. Não demonstra fluidez durante um cálculo
frio pesado; essa frente permanece no passo 7. A GUI do workspace recebeu
alterações concorrentes em outros arquivos, que foram preservadas.

O FSHZ original já tinha a corrente calculada: os microssegundos registrados
nessa leitura são cache, **não** cumprimento da meta de cálculo < 0,1 s.
As metas finais da tabela do plano ainda não foram atingidas.

## Regressão encontrada e correção ainda sem medição

Na `planilha_2m.csv`, a candidata apresentou:

| Medida | Antes | Candidata |
|---|---:|---:|
| Abrir | 15,906 s | 16,218 s |
| Calcular tudo | 6,813 s | 7,104 s |
| Consolidar | 1,756 s | 1,933 s |
| RAM calculada/consolidada | 254,660 MiB | 254,633 MiB |

Uma amostra não separa ruído de regressão; não declarar aprovação.
O caminho de `FormulaTable::store` fazia uma busca extra e
`AtomicBool::swap(false)` para cada resultado, mesmo com coluna limpa.
Foi alterado para aproveitar a busca do slot e usar `load` no caminho comum;
o sinal só recebe `store(false)` ao materializar a invalidação. Há acesso
exclusivo por `&mut self`. **A comparação de desempenho precisa ser refeita.**
Também revisar o custo de `tem_externas` na importação de fórmulas locais.

`spotify_full.csv`: baseline realizada (44,72 s de abertura, 1.135,1 MiB);
comparação depois ainda não realizada. Não afirmar ausência de regressão
nesse corpus.

## Testes e compilação

- Motor completo antes e depois: **682 aprovados, oito falhas preexistentes,
  um ignorado**. As mesmas oito falhas foram reproduzidas no baseline isolado.
- Com exclusão explícita dessas oito falhas: **706 aprovados**, incluindo
  integração do corpus, corrente funda, faixas e sete testes novos de PROCX.
- IO: **152 aprovados**, dois ignorados; inclui round-trip de PROCX compacto
  e gravação antes de recalcular uma fórmula invalidada.
- `cargo check --all-targets --offline` da GUI aprovado, com avisos existentes.
- Após a correção final de `store`, testes PROCX/corrente foram reexecutados;
  resultado em `testes-correcao-store.log`.
- A bancada gráfica temporária foi encerrada. A compilação release normal
  do workspace deve substituir o executável instrumentado antes de encerrar
  a sessão; resultado em `gui-normal-build.log`.

As falhas completas estão em `core-antes.log` e `core-final-completo.log`:
limites de referências, delimitadores de erro, VALOR, limites de deslocamento,
dois testes de spill do histórico, preservação de texto cru e sobreposição
de matrizes. Não foram corrigidas por esta execução e **não há suíte integral
verde**. Os logs filtrados não substituem essa informação.

## Retomar

1. Ler este relatório e `PLANO-MEMORIA-400MB.md`; preservar alterações
   concorrentes e o arquivo `tmp_claude_gravar_real.rs` (não criado aqui).
2. Recompilar a bancada com o código final; repetir pares antes/depois sem
   compilação concorrente, incluindo `planilha_2m` e `spotify_full`.
3. Confirmar a correção de consolidação, avaliar a abertura de CSV e repetir
   abertura/gravação FSHZ após a adaptação de carga.
4. Manter FSH-PERF-001 `[/]` até os gates aplicáveis estarem atendidos.
   Depois seguir o passo 2 (estilo por faixa), sem pular para outra frente.

Bancada reproduzível: `crates/feathersheet-io/examples/medir_plano_unico.rs`:

```bash
cd feathersheet
cargo run --release --offline --manifest-path crates/feathersheet-io/Cargo.toml \
  --example medir_plano_unico -- /caminho/arquivo.xlsx --completo \
  --salvar /tmp/copia-de-medicao.fshz
```

Executáveis e fontes temporários estão em `/tmp/feathersheet-perf-001`:
`antes_plano_final` usa o baseline, `depois_plano_final` a candidata anterior
à correção final de `store`. Eles não devem ser confundidos com o binário
normal do aplicativo. Logs preservados nesta pasta acompanham o relatório.
