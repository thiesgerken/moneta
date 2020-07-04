export const muiDatatablesLang = {
  body: {
    noMatch: 'Keine Daten vorhanden.',
    toolTip: 'Sortieren',
    columnHeaderTooltip: column => `Sortiere nach ${column.label}`,
  },
  pagination: {
    next: 'Nächste Seite',
    previous: 'Vorherige Seite',
    rowsPerPage: 'Zeilen pro Seite:',
    displayRows: 'von',
  },
  toolbar: {
    search: 'Suchen',
    viewColumns: 'Spalten',
    filterTable: 'Filter',
  },
  filter: {
    all: 'Alle',
    title: 'FILTER',
    reset: 'ZURÜCKSETZEN',
  },
  viewColumns: {
    title: 'Angezeigte Spalten',
    titleAria: 'Zeigen/Ausblenden von Tabellenspalten',
  },
  selectedRows: {
    text: 'Zeile(n) ausgewählt',
    delete: 'Löschen',
    deleteAria: 'ausgewählte Zeilen löschen',
  },
};

export const accountAvailabilityToString = r => {
  const translations = {
    immediately: 'Sofort',
    days: 'Tage',
    weeks: 'Wochen',
    months: 'Monate',
    years: 'Jahre',
    decades: 'Jahrzehnte',
  };

  if (r.toLowerCase().trim() in translations)
    return translations[r.toLowerCase().trim()];
  return r;
};

export const accountKindToString = r => {
  const translations = {
    cash: 'Bargeld',
    debit: 'Girokonto',
    credit: 'Kredit',
    debt: 'Schulden',
    stocks: 'Aktien',
    virtual: 'Virtuell',
    other: 'Sonstiges',
  };

  if (r.toLowerCase().trim() in translations)
    return translations[r.toLowerCase().trim()];
  return r;
};

export const accountRiskToString = r => {
  const translations = {
    none: 'Keins',
    slight: 'Sehr niedrig',
    low: 'Niedrig',
    medium: 'Mittel',
    high: 'Hoch',
    huge: 'Sehr hoch',
  };

  if (r.toLowerCase().trim() in translations)
    return translations[r.toLowerCase().trim()];
  return r;
};
