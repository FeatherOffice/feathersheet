# CSV e operações grandes

Atualizado em 09/10/2026.

A detecção automática escolhe **um único separador para o arquivo**:
vírgula, ponto e vírgula ou tabulação. Compara a consistência de até 32
registros de uma amostra limitada, desconsiderando separadores dentro de
aspas. Depois de escolher vírgula, `nota;extra` continua sendo um campo:
nenhuma segunda divisão por ponto e vírgula acontece. Para dividir esse
conteúdo depois, use **Dividir texto** e escolha o separador desejado.

Configurações → Planilha → Detectar separador do arquivo CSV controla
esse comportamento. O padrão das preferências novas é ligado. Uma
preferência manual já salva é preservada. Desligar a detecção usa o
separador do idioma: português `;`, inglês `,`. TSV sempre usa tabulação.
O idioma continua definindo os argumentos das fórmulas e o padrão de
Dividir texto. Casos ambíguos podem ser importados com a escolha manual.

Os limites artificiais de 100 mil foram removidos de copiar/recortar,
colar, pincel de formatação, limpar conteúdo, cópia de intervalo no motor
e perfil de coluna. Os limites dimensionais da planilha continuam
válidos: 100 milhões de linhas e 16.384 colunas. O perfil captura a coluna
selecionada e calcula estatísticas grandes em segundo plano; resultados
cancelados não reabrem o painel. Cópia e colagem permanecem síncronas e
armazenam os dados e estilos necessários à operação.

No motor TypeScript, o dicionário de textos também tinha um teto de 100
mil valores distintos, que podia descartar textos de células e resultados
de fórmulas. Esse teto foi removido; os IDs seguem a capacidade de Int32.
Há regressão com mais de 100 mil textos distintos e resultado textual
de fórmula após a expansão do dicionário.

A importação do disco nativa permanece incremental. A leitura em fatias
TypeScript agora mantém o estado das aspas entre fatias e usa decoder
UTF-8 incremental: campos multilinha, caracteres acentuados e escapes de
aspas não são cortados na fronteira do buffer.

Os testes específicos de CSV, armazenamento, operações acima de 100 mil,
perfil e compilação passaram. A validação geral ainda apresenta falhas em
testes de condicional/geometria de popovers e erro de compilação E0282 no
exemplo `featheroffice-charts/examples/cht018_25_tipos.rs`; o checklist
APP-019 permanece em execução para esses critérios e para aceite visual.
