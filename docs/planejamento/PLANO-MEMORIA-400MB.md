# Plano: o mais econômico com a maior velocidade possível

Pedido do usuário (13/10/2026): o `FeatherSheet_teste_pesado_1M.xlsx`
(1 milhão de linhas × 13 colunas, 7 milhões de fórmulas) gasta **1.100 MB no
Excel** e **~1.900 MB no FeatherSheet**; salvo em `.fshz` e reaberto, **passou
de 4 GB**. Meta: **~400 MB** com a planilha calculada, sem a janela ficar mais
lenta.

## Regra de cada fase

Toda fase só entra se, medida com os arquivos de referência (Fase 0):

- a RAM cair (depois de abrir **e** depois de calcular, e o pico);
- abrir, calcular e salvar **não ficarem mais lentos**;
- rolar e navegar com as setas continuarem abaixo de 16 ms por quadro;
- os testes não ganharem falha nova.

Ganho de memória que deixa o app lento é regressão (lição de 13/10: a
fórmula-com-estilo que dobrou o tempo da corrente).

## Onde está a memória hoje (medido em 13/10, release)

Aba Dados, depois de calcular tudo: 6 M literais, 6 M fórmulas empacotadas,
1 M fórmulas avulsas (`PROCX`, leem outra aba), 6 M estilos por célula,
2 M textos de reserva, 6 M resultados num mapa à parte, 4 M valores de
faixas lidas.

| Estrutura | Aprox. | Por quê |
|---|---|---|
| Resultados no mapa `calc.cache` | ~360 MB | ~60 B por fórmula; a coluna já tem 8 B reservados (resolvido em parte: consolidação no pulso) |
| `PROCX` avulsas | ~300 MB | ~300 B por célula: fórmula que lê outra aba não cabe na coluna compacta |
| Estilo por célula (`styles`) | ~150 MB | 6 M entradas de mapa; a coluna inteira tem o mesmo estilo |
| Textos de reserva (`textos`) | ~100 MB | o leitor do XLSX passa número/data como texto (`"438.99"`, `"29/05/2026"`) e a volta não bate |
| Faixas lidas (`SOMASES`) | ~90 MB | cópia de cada valor (texto clonado); solta no pulso |

No `.fshz`: a leitura monta a pasta inteira numa estrutura intermediária
(`PastaLida`) antes da planilha, e cada estilo de célula vira um objeto JSON
(`estilos_diretos: Vec<(u64, Json)>`) — dezenas de bytes vezes 6 M, mais a
cópia de todas as colunas. É o estouro de 4 GB.

## O piso

Número 8 B; fórmula empacotada = padrão + resultado ≈ 12 B; texto = código de
4 B no dicionário; estilo por faixa ≈ 0. Para 13 M células: **~200 MB de
dados**, mais a janela (Iced, fontes, GPU; a medir na Fase 0). Daí os
~400 MB.

## Fases

### Fase 0 — Bancada de medição (antes de tudo)

- Exemplo `medir_memoria` no `feathersheet-io`: abrir XLSX, abrir `.fshz`,
  salvar `.fshz`, calcular a Resumo, simular rolagem; RSS e pico (`VmHWM`) em
  cada etapa, e o `Sheet::contagem_memoria` por estrutura.
- Arquivos: `FeatherSheet_teste_pesado_1M.xlsx` e `.fshz`, `planilha_2m`,
  `spotify_full` (regra "medir com os arquivos do usuário").
- Medir a RAM do app vazio (piso da janela).
- `contagem_memoria` no Monitor (aba Memória), para o usuário ver por
  estrutura.

### Fase 1 — `.fshz` sem estouro (urgente: é defeito)

1. **Estilo no arquivo como tabela**: estilos distintos uma vez + índice por
   célula (ou faixas, depois da Fase 2). Ler registra cada estilo uma vez
   (`registrar_parcial`), sem JSON por célula.
2. **Ler direto para a planilha, coluna a coluna**: cada coluna Arrow vai
   para o `DataTable`/`FormulaTable` e é solta antes da próxima. Sem
   `PastaLida` inteira na memória: o pico fica perto do final.
3. **Gravar em fluxo também** (o mesmo problema ao salvar).

Espera: abrir o `.fshz` em ~2 s, com a RAM do que a planilha ocupa (sem pico
extra).

### Fase 2 — Estilo por faixa

`Sheet.styles` deixa de ser um mapa por célula e vira, por coluna, uma lista
de faixas (`linhas 2..1000001 = moeda`). Mesma interface por fora
(`get`/`insert`/`remove`/iterar), para não mexer nos ~50 usos. Ganho:
~150 MB, e gravar/abrir mais rápido (o mapa gigante era parte do custo).

### Fase 3 — Dados tipados na leitura do XLSX

O leitor entrega número como número, data como número com formato, texto como
texto: nada de converter para texto e o motor converter de volta. Some a
reserva de textos (~100 MB) e a abertura fica mais rápida.

### Fase 4 — Fórmulas que leem outra aba na coluna compacta

`PROCX(D2;Produtos!$A$2:$A$201;…)`: a parte dentro da aba (`D2`) segue a
dedução por subtração; a parte de fora entra nas "externas" por padrão
(recalcula quando a outra aba muda, como hoje). Ganho: ~300 MB neste
arquivo; em planilha real, quase toda fórmula de busca.

### Fase 5 — Resultado direto na coluna, faixas lidas compactas

- O cálculo grava o resultado na coluna compacta na hora (coluna com
  mutabilidade interior), sem passar pelo mapa — nem o pico temporário de
  ~360 MB.
- Faixas lidas guardam número como `f64` e texto como código do dicionário,
  não `Value` clonado; teto menor.

### Fase 6 — Abrir sob demanda (abaixo do piso)

Com o `.fshz` colunar: carregar só as colunas que a tela e as fórmulas pedem
(`mmap`), o resto quando precisar. Primeira tela em menos de 0,5 s, e a RAM
proporcional ao que está em uso, não ao arquivo.

## Estimativa (a confirmar a cada fase)

| Depois da fase | RAM calculada (aprox.) |
|---|---|
| Hoje | ~1,45–1,9 GB (XLSX); >4 GB (`.fshz`) |
| 1 | `.fshz` ≈ XLSX |
| 2 + 3 | ~1,2 GB |
| 4 | ~0,9 GB |
| 5 | ~0,5–0,6 GB |
| 6 | ~0,4 GB ou menos |

A janela (Iced, fontes, GPU) entra nesse total; se o piso dela passar de
~200 MB, a meta é revista com o número medido na Fase 0.

## Memória e velocidade juntas (decisão do usuário, 13/10/2026)

"Eu quero o mais econômico com a maior velocidade possível." As duas frentes
viram um plano só: dado compacto também é mais rápido (cabe no cache do
processador), e várias fases servem às duas.

App vazio: **127,8 MB** (medido pelo usuário). Hoje, com a Planilha Pesada:
~1.600 MB no app (dados ~1.470 MB); Excel: 1.100 MB.

### Metas (Planilha Pesada, release)

| Medida | Hoje | Meta |
|---|---|---|
| RAM no app, calculada | ~1.600 MB | **~400–500 MB** |
| Abrir `.fshz` | ~9 s | **< 1 s** (primeira tela < 0,5 s) |
| Abrir `.xlsx` | ~17 s | **~5 s** |
| Aba Resumo inteira | ~6 s | **< 0,3 s** |
| Corrente de 1 milhão | ~10 s | **< 0,1 s** |
| Quadro ao rolar/navegar | — | **< 16 ms**, nunca "não está respondendo" |

### Frente de velocidade

- **V1. Cálculo pela ordem das dependências**: montar a ordem uma vez e
  calcular em sequência (e em paralelo onde não há dependência), em vez de
  descobrir recursivamente. Corrente de 1 M: ~10 s → < 0,1 s.
- **V2. Funções de critério agrupadas**: várias `SOMASES`/`CONT.SES` sobre as
  mesmas colunas viram um agrupamento de uma passada (índice por valor do
  critério). Resumo: ~6 s → décimos.
- **V3. Leitor XLSX em fluxo**, sem objeto por linha, entregando valor tipado
  (junto com a Fase 3).
- **V4. Cálculo fora da thread da janela**: o motor calcula em threads
  próprias sobre dados imutáveis; a janela nunca espera conta.
- **V5. Fórmula compilada** (bytecode) em vez de percorrer a árvore.

### Ordem única

| # | Passo | Serve a |
|---|---|---|
| 1 | Fase 4 — fórmula que lê outra aba na coluna compacta | RAM + abrir |
| 2 | Fase 2 — estilo por faixa | RAM + abrir/salvar |
| 3 | V1 + V2 — ordem de dependências e critério agrupado | velocidade |
| 4 | Fase 5 — resultado direto na coluna, faixas compactas | RAM + leitura |
| 5 | Fase 3 + V3 — leitor XLSX tipado e em fluxo | RAM + abrir |
| 6 | Fase 1 (resto) — montar em paralelo, gravar `.fshz` em fluxo | abrir/salvar |
| 7 | V4 — cálculo fora da janela | fluidez |
| 8 | Fase 6 — abrir sob demanda (`mmap`) | RAM + abrir |
| 9 | V5 — fórmula compilada | velocidade |

Cada passo mede **todas** as linhas da tabela de metas antes e depois
(`examples/medir_memoria.rs`, `examples/medir_xlsx_formulas.rs`); entra só se
nenhuma piorar.
