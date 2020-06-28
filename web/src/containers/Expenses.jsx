import { connect } from 'react-redux';
import React, { useEffect, useState } from 'react';
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

  let initialExpense = {
    id: null,
    userId: -1,
    name: '',
    iban: '',
  };
  if (
    expenses.items !== null &&
    expenseId !== null &&
    parseInt(expenseId, 10) >= 0
  ) {
    const a = find(expenses.items, x => x.id === parseInt(expenseId, 10));
    if (a !== undefined) initialExpense = a;
  }

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
    updateExpenseInfo();
    updateExpenses(currentQuery);
  });

  const columns = [
    {
      name: 'info.id',
      label: 'ID',
      options: {
        display: false,
      },
    },
    {
      name: 'info.title',
      label: 'Titel',
    },
    {
      name: 'info.bookingStart',
      label: 'Buchungszeitraum',
      options: {
        sortOrder: 'desc',
      },
    },
    {
      name: 'info.description',
      label: 'Beschreibung',
    },
    {
      name: 'info.comments',
      label: 'Kommentare',
    },
    {
      name: 'info.store',
      label: 'Geschäft',
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
      // console.log(action, tableState);

      const newQuery = {
        page: tableState.page,
        rowsPerPage: tableState.rowsPerPage,
        filterBy: {},
        sortBy: [],
        needle: tableState.searchText,
        from: null,
        to: null,
      };

      if (JSON.stringify(newQuery) !== JSON.stringify(currentQuery))
        setQuery([currentQuery, newQuery]);

      // a developer could react to change on an action basis or
      // examine the state as a whole and do whatever they want

      // switch (action) {
      //   case 'changePage':
      //     this.changePage(tableState.page, tableState.sortOrder);
      //     break;
      //   case 'sort':
      //     this.sort(tableState.page, tableState.sortOrder);
      //     break;
      //   default:
      //     console.log('action not handled.');
      // }
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
    items: PropTypes.arrayOf(PropTypes.object),
    isFetching: PropTypes.bool,
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
