# Exemplos

## `planilha-pesada.plz` / `planilha-pesada.json`

Uma pasta feita **só de fórmulas** — 22 mil delas —, do tipo que no Excel deixa
o arquivo lento. Abra pelo menu **Arquivo › Abrir…** (o `.plz` é o mesmo
conteúdo, 11× menor).

| Aba | O que tem |
| --- | --- |
| **Dados** | 2.000 linhas × 9 colunas geradas por fórmula: categoria e vendedor por `ÍNDICE` sobre matriz literal, data por `DATA`, e valores encadeados (custo depende do valor, margem do custo, lucro da margem). Nenhum número digitado. |
| **Painel** | Resumo da base inteira: `SOMASES`, `CONT.SES`, `MÉDIASES` e `SOMARPRODUTO` com duas condições por categoria; top 5 com `MAIOR(…;{1;2;3;4;5})`; `ÚNICO`, `FILTRO`+`CLASSIFICAR`, histograma com `FREQÜÊNCIA` e `SEQUÊNCIA`; correlação, inclinação, R², percentil e quartil; `DATADIF` no período. |
| **Matriz** | Covariância 4×4 de 2.000 linhas montada na unha (`TRANSPOR` + `MATRIZ.MULT`), a inversa dela, o determinante, os pesos de variância mínima e a conferência covariância × inversa = identidade. |
| **Cadeia** | 1.000 células em fila indiana (cada uma depende da anterior) e uma soma acumulada `SOMA(A$2:An)` — meio milhão de leituras de célula, o padrão clássico de planilha lenta. |

Para gerar de novo, ou mudar o tamanho (`LINHAS`, no alto do arquivo):
