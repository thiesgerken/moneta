import { connect } from 'react-redux';
import React, { useEffect } from 'react';
import PropTypes from 'prop-types';
import MUIDataTable from 'mui-datatables';
import { find } from 'lodash';
import {
  ThemeProvider,
  createMuiTheme,
  useTheme,
  useMediaQuery,
  Container,
  CircularProgress,
  Tooltip,
  IconButton,
} from '@material-ui/core';
import AddIcon from '@material-ui/icons/Add';
import { useLocation, useHistory } from 'react-router';

import { updateBalances as updateBalancesAction } from '../actions/balances';
import BalanceDialog from './BalanceDialog';

function useQuery(key) {
  const location = useLocation();
  const query = new URLSearchParams(location.search);
  const history = useHistory();

  const set = value => {
    if (value === null || value === undefined) query.delete(key);
    else query.set(key, value);

    const qs = query.toString();
    history.push(`${location.pathname}${qs.length ? '?' : ''}${qs}`);
  };

  return [query.get(key), set];
}

const Balances = ({ balances, updateBalances }) => {
  const [balanceId, setBalanceId] = useQuery('id');
  const [page, setPage] = useQuery('page');

  let initialBalance = {
    id: null,
    userId: -1,
    name: '',
    iban: '',
  };
  if (
    balances.items !== null &&
    balanceId !== null &&
    parseInt(balanceId, 10) >= 0
  ) {
    const a = find(balances.items, x => x.id === parseInt(balanceId, 10));
    if (a !== undefined) initialBalance = a;
  }

  const theme = useTheme();
  const mdUp = useMediaQuery(theme.breakpoints.up('md'));

  const myTheme = th =>
    createMuiTheme({
      ...th,
      overrides: {
        ...th.overrides,
        MUIDataTableSelectCell: {
          root: {
            display: 'none',
          },
        },
      },
    });

  useEffect(() => {
    updateBalances();
  });

  const isLoading = balances.isFetching;

  const columns = [
    {
      name: 'id',
      label: 'ID',
      options: {
        display: false,
      },
    },
    {
      name: 'accountId',
      label: 'Konto',
    },
    {
      name: 'date',
      label: 'Datum',
      options: {
        sortOrder: 'desc',
      },
    },
    {
      name: 'amount',
      label: 'Betrag',
    },
    {
      name: 'comment',
      label: 'Kommentare',
    },
  ];

  const options = {
    selectableRows: 'single',
    selectableRowsOnClick: true,
    selectToolbarPlacement: 'none',
    enableNestedDataAccess: '.',
    print: false,
    page: page === null ? 0 : parseInt(page, 10), // https://github.com/gregnb/mui-datatables/issues/957
    onChangePage: p => {
      setPage(p === 0 ? null : p);
    },
    onRowSelectionChange: (_, rows) => {
      if (rows.length > 0 && rows[0].dataIndex < balances.items.length)
        setBalanceId(balances.items[rows[0].dataIndex].id);
      else setBalanceId(null);
    },
    download: false,
    filter: false,
    rowsPerPageOptions: mdUp ? [10, 25, 50] : [10],
    responsive: 'standard',
    customToolbar: () => (
      <>
        <Tooltip title="Neuer Kontostand">
          <IconButton onClick={() => setBalanceId(-1)}>
            <AddIcon />
          </IconButton>
        </Tooltip>
      </>
    ),
    textLabels: {
      body: {
        noMatch: 'Keine Kontostände vorhanden.',
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
        reset: 'RESET',
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
    },
  };
  return (
    <div style={{ margin: '-4px' }}>
      <Container maxWidth="xl" style={{ padding: '0px' }}>
        <div style={{ position: 'relative', height: '100%', width: '100%' }}>
          {isLoading && (
            <div
              style={{
                position: 'absolute',
                left: '50%',
                top: '50%',
                transform: 'translate(0%, -0%)',
                width: '0%',
                height: '0%',
                padding: '0px',
                margin: 0,
                zIndex: 9,
              }}
            >
              <CircularProgress color="secondary" />
            </div>
          )}
          <div
            style={{
              position: 'relative',
              width: '100%',
              height: '100%',
              left: '0',
              top: '0',
            }}
          >
            <ThemeProvider theme={myTheme}>
              <MUIDataTable
                title="Kontostände"
                data={balances.items}
                columns={columns}
                options={options}
              />
            </ThemeProvider>
          </div>
        </div>
      </Container>
      <BalanceDialog
        initialBalance={initialBalance}
        open={balanceId !== null && !isLoading}
        onClose={() => {
          setBalanceId(null);
        }}
      />
    </div>
  );
};

Balances.propTypes = {
  updateBalances: PropTypes.func.isRequired,
  balances: PropTypes.shape({
    items: PropTypes.arrayOf(PropTypes.object),
    isFetching: PropTypes.bool,
  }).isRequired,
};

const mapDispatchToProps = dispatch => ({
  updateBalances: () => dispatch(updateBalancesAction()),
});

const mapStateToProps = state => {
  const { balances } = state;

  return {
    balances,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Balances);
