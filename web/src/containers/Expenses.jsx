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
import { forEach, map, sortBy, uniqBy, zip } from 'lodash';

import {
  updateExpenses as updateExpensesAction,
  updateExpenseInfo as updateExpenseInfoAction,
} from '../actions/expenses';
import { updateAccounts as updateAccountsAction } from '../actions/accounts';
import { updateCategories as updateCategoriesAction } from '../actions/categories';
import ExpenseDialog from './ExpenseDialog';
import { muiDatatablesLang } from '../lang';
import { findExpenseQueryData } from '../selectors/expenses';
import { findAccount } from '../selectors/accounts';
import { findCategory } from '../selectors/categories';

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

const Expenses = ({
  expenses,
  updateExpenses,
  updateExpenseInfo,
  accounts,
  updateAccounts,
  categories,
  updateCategories,
  userInfo,
}) => {
  const [expenseId, setExpenseId] = useQuery('id');
  const [query, setQuery] = useState([
    null,
    {
      page: 0,
      rowsPerPage: 10,
      filterBy: {},
      sortBy: [{ column: 'info.bookingEnd', direction: 'descending' }],
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
  // let displayedQuery = currentQuery;
  let displayedResponse = currentQueryResponse;

  if (prevQuery && isLoading) {
    // continue displaying the old data for now (and a loading indicator)
    const prevQueryData = findExpenseQueryData(expenses, prevQuery);
    const prevQueryResponse =
      prevQueryData && prevQueryData.data
        ? prevQueryData.data
        : { records: [], filteredRecordCount: 0, totalRecordCount: null };

    // displayedQuery = prevQuery;
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
    updateAccounts();
    updateCategories();
    updateExpenseInfo();
    updateExpenses(currentQuery);
  });

  // TODO: how to control column widths?
  // TODO: add ellipses in text fields

  const columns = [
    {
      name: 'info.id',
      label: 'ID',
      options: {
        display: false,
        filterType: 'textField',
        customFilterListOptions: { render: v => `ID: ${v}` },
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
      // TODO: rendering of categories: set background color to gradient
      // TODO: filter values for categories
      name: 'categories',
      label: 'Kategorien',
      options: {
        customBodyRenderLite: i => {
          const cs = uniqBy(
            sortBy(displayedResponse.records[i].categories, [
              c => -1 * c.weight,
            ]),
            t => t.categoryId
          );

          return map(cs, c => {
            const cat = findCategory(categories, c.categoryId);
            if (!cat) return `[${c.categoryId}]`;

            // TODO: find out username of the other user
            if (userInfo && cat.info.userId === userInfo.id)
              return cat.info.name;
            return `${cat.info.userId}/${cat.info.name}`;
          }).join(', ');
        },
        sort: false,
      },
    },
    {
      // TODO: rendering of accounts: set background color to gradient
      // TODO: filter values for accounts
      name: 'transactions',
      label: 'Konten',
      options: {
        customBodyRenderLite: i => {
          // TODO: first sort by accountId, sum the calculatedAmounts of transactions belonging to the same account id
          // (do something similar in setCellProps)
          const ts = uniqBy(
            sortBy(
              zip(
                displayedResponse.records[i].transactions,
                displayedResponse.records[i].calculatedAmounts
              ),
              [t => Math.abs(t[1])]
            ),
            t => t[0].accountId
          );

          return map(ts, t => {
            const acc = findAccount(accounts, t[0].accountId);
            if (!acc) return `[${t[0].accountId}]`;

            // TODO: find out username of the other user
            if (userInfo && acc.info.userId === userInfo.id)
              return acc.info.name;
            return `${acc.info.userId}/${acc.info.name}`;
          }).join(', ');
        },
        sort: false,
        setCellProps: (_, i) => {
          console.log(i);
          return {};
          // if (data.sharing.length == 0) {
          //   Util.setBackgroundColor(rowjq.find('td:nth-child(4)'), data.account.color, false);
          // } else {
          //   let colors = [data.account.color];
          //   let w1 = Math.abs(data.amount);
          //   if (w1 == 0) w1 = 1;
          //   let weights = [w1];
          //   let sums = [0, w1];

          //   for (let j = 0; j < data.sharing.length; j++) {
          //     let w = Math.abs(data.sharing[j].calculatedAmount);
          //     if (w == 0) w = 1;

          //     colors.push(data.sharing[j].account.color);
          //     weights.push(w);
          //     sums.push(sums[sums.length - 1] + weights[weights.length - 1]);
          //   }

          //   let totalWeight = weights.reduce((a, b) => a + b, 0);
          //   let s = 'border-style: none; background-image: linear-gradient(to right, ';

          //   for (let i = 0; i < colors.length; i++) {
          //     if (i != 0) s += ',';
          //     s += colors[i];

          //     if (i != 0) {
          //       s += ' ' + (sums[i] / totalWeight * 100).toFixed(0) + '%';
          //     }

          //     if (i != colors.length - 1) {
          //       s += ', ' + colors[i] + ' ' + (sums[i + 1] / totalWeight * 100).toFixed(0) + '%';
          //     }
          //   }
          //   s += ');';

          //   rowjq.find('td:nth-child(4)').attr('style', s);
          // }
        },
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
        filterOptions: {
          names: expenses.info.data ? expenses.info.data.filterHints.store : [],
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
        customFilterListOptions: { render: () => 'Vorläufig' },
      },
    },
    {
      name: 'info.isTaxRelevant',
      label: 'Steuerrelevant',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => 'Steuerrelevant' },
      },
    },
    {
      name: 'info.isTemplate',
      label: 'Vorlage',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => 'Vorlage' },
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

  const currentSortOrder = {};
  if (currentQuery.sortBy.length > 0) {
    currentSortOrder.direction =
      currentQuery.sortBy[0].direction === 'ascending' ? 'asc' : 'desc';
    currentSortOrder.name = currentQuery.sortBy[0].column;
  }

  const options = {
    resizableColumns: true,
    selectableRows: 'single',
    selectableRowsOnClick: true,
    selectToolbarPlacement: 'none',
    enableNestedDataAccess: '.',
    serverSide: true,
    print: false,
    onTableChange: (action, tableState) => {
      console.log(action, tableState);

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
        const column = tableState.sortOrder.name;

        newQuery.sortBy.push({
          direction:
            tableState.sortOrder.direction === 'asc'
              ? 'ascending'
              : 'descending',
          column,
        });
      }

      forEach(tableState.filterList, (v, i) => {
        if (v.length > 0) {
          newQuery.filterBy[columns[i].name] = map(v, x =>
            typeof x === 'boolean' ? (!x).toString() : x
          );
        }
      });

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
    rowsPerPage: currentQuery.rowsPerPage,
    page: currentQuery.page,
    count: currentQueryResponse.filteredRecordCount,
    sortOrder: currentSortOrder,
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

Expenses.defaultProps = { userInfo: null };

Expenses.propTypes = {
  updateExpenses: PropTypes.func.isRequired,
  updateExpenseInfo: PropTypes.func.isRequired,
  updateCategories: PropTypes.func.isRequired,
  updateAccounts: PropTypes.func.isRequired,
  expenses: PropTypes.shape({
    info: PropTypes.shape({
      isFetching: PropTypes.bool.isRequired,
      data: PropTypes.shape({
        filterHints: PropTypes.shape({
          store: PropTypes.arrayOf(PropTypes.string),
        }),
      }),
    }).isRequired,
    queries: PropTypes.arrayOf(
      PropTypes.shape({
        isFetching: PropTypes.bool.isRequired,
        // eslint-disable-next-line react/forbid-prop-types
        data: PropTypes.object,
        // eslint-disable-next-line react/forbid-prop-types
        query: PropTypes.object.isRequired,
      })
    ),
  }).isRequired,
  accounts: PropTypes.shape({
    isFetching: PropTypes.bool.isRequired,
    items: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
  categories: PropTypes.shape({
    isFetching: PropTypes.bool.isRequired,
    items: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
  userInfo: PropTypes.shape({
    id: PropTypes.number.isRequired,
  }),
};

const mapDispatchToProps = dispatch => ({
  updateExpenses: query => dispatch(updateExpensesAction(query)),
  updateExpenseInfo: () => dispatch(updateExpenseInfoAction()),
  updateAccounts: () => dispatch(updateAccountsAction()),
  updateCategories: () => dispatch(updateCategoriesAction()),
});

const mapStateToProps = state => {
  const { expenses, accounts, categories, authentication } = state;
  const { userInfo } = authentication;

  return {
    expenses,
    accounts,
    categories,
    userInfo,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Expenses);
