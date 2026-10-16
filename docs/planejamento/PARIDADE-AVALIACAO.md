# Paridade com o Excel — avaliação geral

> Avaliação realizada em 13/10/2026. O checklist item a item está em
> [`PARIDADE-EXCEL.md`](PARIDADE-EXCEL.md). Números marcados como **medido** vieram de
> testes com arquivos reais; as demais informações foram extraídas do código.
> Prioridades do FeatherSheet: arquivos grandes, OpenQuery, macros e suporte a pt-BR.

## Resumo

| Medida | FeatherSheet | Excel |
|---|---|---|
| Comandos da faixa (amplitude) | **187 / 290 (64,5%)** | 290 |
| Comandos da aba Desenvolvedor (macros) | **4 / 19** (fora do total acima) | 19 |
| Funções de planilha | **457 / 520** nomes do Excel no motor, 87,9% (atualizado em 15/10/2026: +48 neste lote; lista do que falta em [`FUNCOES-FALTANTES.md`](FUNCOES-FALTANTES.md)) | 523 |
| Linguagem / interface | Rust + Iced | C++ |
| Formato próprio | `.fshz` (colunar, Arrow) | XLSX |

Onde estamos atrás: Inserir (formas, minigráficos, comentários), Revisão (1 / 18: ortografia,
comentários, anotações, proteção de pasta), Layout da Página (temas, margens, organizar objetos),
Desenvolvedor (4 / 19: gravar macro, suplementos, controles e XML), Dados (previsão, Solver,
consolidar). A faixa está perto na Página Inicial (85 / 94) e na tabela dinâmica (14 / 17).

Macros e VBA (conversor, macros reais e eventos, com números medidos): ver a seção
[Macros e VBA](#macros-e-vba) abaixo.

## Escala e desempenho

O FeatherSheet prioriza arquivos grandes, uso eficiente da memória e operações em tabelas
acima do limite de linhas do Excel.

| Medida | FeatherSheet | Excel | Tipo |
|---|---|---|---|
| Tamanho máximo da planilha | **100.000.000 × 16.384** (`celulas/refs.rs`) | 1.048.576 × 16.384 | código |
| Armazenamento | colunar (`DataTable`, texto num dicionário, fórmula por padrão: 12 bytes por célula) | — | código |
| Abrir CSV de 817 MB (1,18 M linhas × 83 colunas, 87,6 M células) | 51,5 s, pico 1,1 GB | corta em 1.048.576 linhas | **medido** |
| Abrir `.fshz` de 126 MB (Spotify) | ~4,6 s (ler 1,5 s + montar 3 s), ~0,73 GB | — | **medido** |
| Abrir XLSX de ~100 MB | **29 s, interface responsiva** | abre | **relato do usuário, 13/10/2026** |
| CSV do Spotify aberto + Texto para Colunas (mesma operação nos dois) | **1.623 MB**, mais rápido e sem travar o computador | ~4.500 MB | **medido pelo usuário, 13/10/2026** |
| Texto para Colunas, 87,6 M células | **24 s**, pico 2,8 GB (era 108 s) | — | **medido** |
| Desfazer do Texto para Colunas acima | 6 s (foto só do que existia) | — | **medido** |
| Achar a região atual (Ctrl+T, Ctrl+A) em 552 mil linhas × 83 | 0,012 s | — | **medido** |
| Localizar em 1,18 M × 82 | 0,64 s (paralelo por coluna) | — | medido em 23/09 |
| Uso de vários núcleos | busca por coluna; corte do Texto para Colunas; cálculo das fórmulas | recalc em todos | código |
| Recálculo em vários núcleos | **sim**: colunas em níveis de dependência, todos os núcleos menos um (Planilha Pesada 25,9 → 5,5 s) | sim | **medido** 13/10 |
| Grade | virtualizada | — | código |

> Medição relatada pelo usuário em sua máquina, sem registro de pico de RAM ou configuração do arquivo; não é benchmark reproduzido neste ambiente.

## Lacunas de desempenho conhecidas

- **Gravar célula por célula ainda custa ~190 ns**: no Texto para Colunas ficam 16,6 s só de
  gravação. Próximo passo: gravar coluna inteira de uma vez e paralelizar por coluna.
- **Montar a pasta depois de ler o `.fshz`** leva 3 s — maior parte da abertura.
- **19 milhões de textos originais guardados** no Spotify (`False`, `4.0`, `5.15e-06`) para não
  perder a grafia do arquivo: ~400 MB de mapa de chaves. Decidir com o usuário se vale guardar.

## Macros e VBA

Medidas da parte de macros, com o VBA do Excel como referência. Os números de execução vieram
de testes com arquivos reais (`macros_reais_rodam`, 14/10/2026).

| Medida | FeatherSheet | Excel | Tipo |
|---|---|---|---|
| Comandos da aba Desenvolvedor | **4 / 19** (ver [`PARIDADE-EXCEL.md`](PARIDADE-EXCEL.md), "Desenvolvedor (macros)") | 19 | código |
| Funções do VBA traduzidas pelo conversor | **52 cobertas + 12 parciais**, de 117 aplicáveis (**~55%**); 24 não se aplicam (Windows, Office, arquivo, impressora) | — | levantamento de 14/10 |
| Planilha v6.0a (`.xlsm`) | **24 de 28** macros rodando (falham 3 do Word e `copiarinversoresmodulos`) | — | **medido** |
| precioso.xlsm | **17 de 18** rodando (falha: laço de 100 mil voltas) | — | **medido** |
| Fluxo de caixa.xlsm | **7 de 8** rodando (falha: máscara `"dd"`) | — | **medido** |
| Eventos `Worksheet_Change` e `Worksheet_Activate` | **rodam sozinhos** (aoMudar, aoAtivar; `6edc796e`) | Excel: rodam | código |
| `Workbook_Open` | **à mão** (de propósito: é o caso mais arriscado) | Excel: roda ao abrir | código |
| Executar VBA | **sim**, convertido para JavaScript, na máquina (o VBA original não roda) | Excel roda VBA | — |

**Funções do VBA que mais faltam** (as 15 mais usadas em planilha, pelo levantamento de 14/10):
CDate, DateAdd, DateDiff, DateSerial, DateValue, Weekday, Hour, Minute, Second, IsDate, Choose,
Switch, IsError, FormatNumber (com FormatCurrency e FormatPercent), TypeName (com VarType). As
financeiras (FV, PV, Pmt, NPer, Rate…) também faltam, mas em planilha costumam ir para a fórmula.

Fonte do levantamento de funções: [`PARIDADE-VBA-FUNCOES.md`](PARIDADE-VBA-FUNCOES.md) (14/10/2026).

## Como manter

Ao ligar um comando novo na faixa, mover o item de "Faltando" para "Implementado" em
`PARIDADE-EXCEL.md` (com o lugar onde ele fica) e acertar as contagens. Ao medir algo novo, pôr a
linha na tabela de desempenho com a data e "medido".
