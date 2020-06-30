import { connect } from 'react-redux';
import React, { useEffect, useState } from 'react';
import PropTypes from 'prop-types';
import MUIDataTable from 'mui-datatables';
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
import * as moment from 'moment';
import { map } from 'lodash';

import {
  updateExpenses as updateExpensesAction,
  updateExpenseInfo as updateExpenseInfoAction,
} from '../actions/expenses';
import ExpenseDialog from './ExpenseDialog';
import { muiDatatablesLang } from '../lang';
import { findExpenseQueryData } from '../selectors/expenses';

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

const Expenses = ({ expenses, updateExpenses, updateExpenseInfo }) => {
  const [expenseId, setExpenseId] = useQuery('id');
  // const [page, setPage] = useQuery('page');
  const [query, setQuery] = useState([
    null,
    {
      page: 0,
      rowsPerPage: 10,
      filterBy: {},
      sortBy: [],
      needle: null,
      from: null,
      to: null,
    },
  ]);
  const [prevQuery, currentQuery] = query;

  const currentQueryData = findExpenseQueryData(expenses, currentQuery);
  const currentQueryResponse =
    currentQueryData && currentQueryData.data
      ? currentQueryData.data
      : { records: [], filteredRecordCount: 0, totalRecordCount: null };

  const isLoading = !currentQueryData || currentQueryData.isFetching;
  let displayedQuery = currentQuery;
  let displayedResponse = currentQueryResponse;

  if (prevQuery && isLoading) {
    // continue displaying the old data for now (and a loading indicator)
    const prevQueryData = findExpenseQueryData(expenses, prevQuery);
    const prevQueryResponse =
      prevQueryData && prevQueryData.data
        ? prevQueryData.data
        : { records: [], filteredRecordCount: 0, totalRecordCount: null };

    displayedQuery = prevQuery;
    displayedResponse = prevQueryResponse;
  }

  const initialExpense = {
    id: null,
    userId: -1,
    name: '',
    iban: '',
  };
  // if (
  //   expenses.items !== null &&
  //   expenseId !== null &&
  //   parseInt(expenseId, 10) >= 0
  // ) {
  //   const a = find(expenses.items, x => x.id === parseInt(expenseId, 10));
  //   if (a !== undefined) initialExpense = a;
  // }

  const theme = useTheme();
  const mdUp = useMediaQuery(theme.breakpoints.up('md'));
  const xsDown = useMediaQuery(theme.breakpoints.down('xs'));

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
    updateExpenseInfo();
    updateExpenses(currentQuery);
  });

  const columns = [
    {
      name: 'info.id',
      label: 'ID',
      options: {
        display: false,
        filterType: 'textField',
      },
    },
    {
      name: 'info.bookingEnd',
      label: 'Buchungszeitraum',
      options: {
        filter: false, // TODO: one could integrate from-to filtering here!
        customBodyRenderLite: i => {
          const r = displayedResponse.records[i];
          const st = moment(r.info.bookingStart);
          const en = moment(r.info.bookingEnd);

          if (xsDown) return st.format('L');

          if (st.isSame(en)) return st.format('L LT');
          if (st.isSame(en, 'day'))
            return `${st.format('L LT')} - ${en.format('LT')}`;
          return `${st.format('L')} - ${en.format('L')}`;
        },
      },
    },
    {
      name: 'info.title',
      label: 'Titel',
      options: {
        filter: false,
      },
    },
    {
      // TODO: rendering of categories
      // TODO: filter values for categories
      name: 'categories',
      label: 'Kategorien',
      options: {
        customBodyRenderLite: i => {
          return map(
            displayedResponse.records[i].categories,
            c => c.categoryId
          ).join(', ');
        },
        sort: false,
      },
    },
    {
      // TODO: rendering of accounts
      // TODO: filter values for accounts
      name: 'transactions',
      label: 'Konten',
      options: {
        customBodyRenderLite: i => {
          return map(
            displayedResponse.records[i].transactions,
            t => t.accountId
          ).join(', ');
        },
        sort: false,
      },
    },
    {
      name: 'totalAmount',
      label: 'Betrag',
      options: {
        // TODO: highlight amount depending on sign
        customBodyRenderLite: i =>
          `${(displayedResponse.records[i].totalAmount / 100.0).toFixed(2)}€`,
        filter: false,
        sort: false,
      },
    },
    {
      name: 'info.store',
      label: 'Geschäft',
      options: {
        filter: true,
        // filterType: 'textField',
        filterOptions: {
          names: expenses.info.data ? expenses.info.data.filterHints.store : [],
          fullWidth: true,
        },
      },
    },
    {
      name: 'info.description',
      label: 'Beschreibung',
      options: {
        filter: false,
      },
    },
    {
      name: 'info.comments',
      label: 'Kommentare',
      options: {
        filter: false,
      },
    },
    {
      name: 'info.isPreliminary',
      label: 'Vorläufig',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => `Vorläufig` },
      },
    },
    {
      name: 'info.isTaxRelevant',
      label: 'Steuerrelevant',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => `Steuerrelevant` },
      },
    },
    {
      name: 'info.isTemplate',
      label: 'Vorlage',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => `Vorlage` },
      },
    },
    {
      name: 'info.isUnchecked',
      label: 'Unüberprüft',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => `nicht überprüft` },
      },
    },
  ];

  const options = {
    selectableRows: 'single',
    selectableRowsOnClick: true,
    selectToolbarPlacement: 'none',
    enableNestedDataAccess: '.',
    serverSide: true,
    print: false,
    // page: page === null ? 0 : parseInt(page, 10), // https://github.com/gregnb/mui-datatables/issues/957
    // onChangePage: p => {
    //   setPage(p === 0 ? null : p);
    // },
    onTableChange: (action, tableState) => {
      console.log(action, tableState);

      // TODO: default sorting!

      const newQuery = {
        page: tableState.page,
        rowsPerPage: tableState.rowsPerPage,
        filterBy: {},
        sortBy: [],
        needle: tableState.searchText,
        from: null,
        to: null,
      };

      if (tableState.sortOrder.direction) {
        let column = tableState.sortOrder.name;
        if (column.startsWith('info.')) column = column.substring(5);

        newQuery.sortBy.push({
          direction:
            tableState.sortOrder.direction === 'asc'
              ? 'ascending'
              : 'descending',
          column,
        });
      }

      // TODO: filtering

      if (JSON.stringify(newQuery) !== JSON.stringify(currentQuery))
        setQuery([currentQuery, newQuery]);
    },
    // onRowSelectionChange: (_, rows) => {
    //   if (rows.length > 0 && rows[0].dataIndex < expenses.items.length)
    //     setExpenseId(expenses.items[rows[0].dataIndex].info.id);
    //   else setExpenseId(null);
    // },
    download: false,
    rowsPerPageOptions: mdUp ? [10, 25, 50] : [10],
    rowsPerPage: displayedQuery.rowsPerPage,
    page: displayedQuery.page,
    count: displayedResponse.filteredRecordCount,
    filterType: 'multiselect',
    responsive: 'standard',
    customToolbar: () => (
      <>
        <Tooltip title="Neue Ausgabe">
          <IconButton onClick={() => setExpenseId(-1)}>
            <AddIcon />
          </IconButton>
        </Tooltip>
      </>
    ),
    textLabels: muiDatatablesLang,
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
                title="Ausgaben"
                data={displayedResponse.records}
                columns={columns}
                options={options}
              />
            </ThemeProvider>
          </div>
        </div>
      </Container>
      <ExpenseDialog
        initialExpense={initialExpense}
        open={expenseId !== null && !isLoading}
        onClose={() => {
          setExpenseId(null);
        }}
      />
    </div>
  );
};

Expenses.propTypes = {
  updateExpenses: PropTypes.func.isRequired,
  updateExpenseInfo: PropTypes.func.isRequired,
  expenses: PropTypes.shape({
    info: PropTypes.object,
    queries: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
};

const mapDispatchToProps = dispatch => ({
  updateExpenses: query => dispatch(updateExpensesAction(query)),
  updateExpenseInfo: () => dispatch(updateExpenseInfoAction()),
});

const mapStateToProps = state => {
  const { expenses } = state;

  return {
    expenses,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Expenses);
