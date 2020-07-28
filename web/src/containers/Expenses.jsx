import { connect } from 'react-redux';
import React, { useEffect, useState } from 'react';
import PropTypes from 'prop-types';
import MUIDataTable from 'mui-datatables';
import WarningIcon from '@material-ui/icons/Warning';
import {
  ThemeProvider,
  createMuiTheme,
  useTheme,
  useMediaQuery,
  Container,
  CircularProgress,
  Tooltip,
  IconButton,
  Typography,
  TableRow,
  TableCell,
} from '@material-ui/core';
import AddIcon from '@material-ui/icons/Add';
import EditIcon from '@material-ui/icons/Edit';
import { useLocation, useHistory } from 'react-router';
import { forEach, map, sortBy, uniqBy, filter } from 'lodash';
import tinycolor from 'tinycolor2';
import green from '@material-ui/core/colors/green';
import deepOrange from '@material-ui/core/colors/deepOrange';

import {
  updateExpenses as updateExpensesAction,
  updateExpenseInfo as updateExpenseInfoAction,
} from '../actions/expenses';
import { updateAccounts as updateAccountsAction } from '../actions/accounts';
import { updateUsers as updateUsersAction } from '../actions/users';
import { updateCategories as updateCategoriesAction } from '../actions/categories';
import ExpenseDialog from './ExpenseDialog';
import { muiDatatablesLang } from '../lang';
import { findExpenseQueryData, formatBooking } from '../selectors/expenses';
import {
  findAccount,
  findAccountByName,
  displayAccountById,
} from '../selectors/accounts';
import {
  findCategory,
  findCategoryByName,
  displayCategoryById,
} from '../selectors/categories';
import ExpenseDetails from '../components/ExpenseDetails';

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
  users,
  updateUsers,
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
      rowsPerPage: 15,
      filterBy: {},
      sortBy: [{ column: 'info.bookingEnd', direction: 'descending' }],
      needle: null,
      from: null,
      to: null,
    },
  ]);
  const [expandedRow, setExpandedRow] = useState(null);
  const [selectedRow, setSelectedRow] = useState(null);
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

  const cellStyle = {
    textOverflow: 'ellipsis',
    overflow: 'hidden',
    display: '-webkit-box',
    WebkitLineClamp: xsDown ? 2 : 1,
    WebkitBoxOrient: 'vertical',
  };

  // TODO: I don't like this way of fixing column widths!

  const myTheme = th =>
    createMuiTheme({
      ...th,
      overrides: {
        ...th.overrides,
        MUIDataTableHeadCell: {
          root: {
            '&:nth-child(1)': {
              width: '0%',
            },
            '&:nth-child(2)': {
              width: '2%',
            },
            '&:nth-child(3)': {
              width: '8%',
            },
            '&:nth-child(4)': {
              width: '15%',
            },
            '&:nth-child(5)': {
              width: '12%',
            },
            '&:nth-child(6)': {
              width: '15%',
            },
            '&:nth-child(7)': {
              width: '8%',
            },
            '&:nth-child(8)': {
              width: '10%',
            },
            '&:nth-child(9)': {
              width: '20%',
            },
            '&:nth-child(10)': {
              width: '10%',
            },
          },
        },
        MUIDataTableBodyCell: {
          root: { '& div': cellStyle },
        },
        MUIDataTableSelectCell: {
          root: {
            display: 'none',
          },
        },
      },
    });

  useEffect(() => {
    updateUsers();
    updateAccounts();
    updateCategories();
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
        customFilterListOptions: { render: v => `ID: ${v}` },
      },
    },
    {
      name: 'info',
      label: ' ',
      options: {
        viewColumns: false,
        sort: false,
        filter: false,
        customBodyRenderLite: i => {
          // TODO: I would like this Icon to be bigger without messing with the row height.
          const r = displayedResponse.records[i];
          if (r.info.isUnchecked)
            return (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                }}
              >
                <WarningIcon fontSize="inherit" color="secondary" />
              </div>
            );

          return '';
        },
      },
    },
    {
      name: 'info.bookingEnd',
      label: mdUp ? 'Buchungszeitraum' : 'Datum',
      options: {
        filter: false, // TODO: one could integrate from-to filtering here!
        customBodyRenderLite: i => {
          return formatBooking(displayedResponse.records[i], xsDown);
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

          return (
            <div style={{ margin: theme.spacing(2) }}>
              {map(cs, c =>
                displayCategoryById(categories, users, userInfo, c.categoryId)
              ).join(', ')}
            </div>
          );
        },
        filterOptions: {
          names: map(
            userInfo
              ? filter(categories.items, c => c.info.userId === userInfo.id)
              : [],
            c => c.info.name
          ),
        },
        sort: false,
        setCellProps: (_, i) => {
          const data = map(displayedResponse.records[i].categories, x => ({
            ...x,
            color:
              findCategory(categories, x.categoryId)?.info?.color ||
              '#FFFFFF00',
          }));

          const style = { padding: 0 };
          if (data.length === 0) return { style };

          if (
            tinycolor(data[0].color).isValid() &&
            tinycolor(data[0].color).getBrightness() <= 120
          )
            style.color = '#F7F7F7';

          if (data.length === 1) {
            style.backgroundColor = data[0].color;
          } else {
            const colors = [];
            const weights = [];
            const sums = [0];

            data.forEach(d => {
              const w = Math.abs(d.amount);
              colors.push(d.color);
              weights.push(w);
              sums.push(sums[sums.length - 1] + w);
            });

            const totalWeight = weights.reduce((a, b) => a + b, 0);
            const segments = [];

            for (let j = 0; j < colors.length; j += 1) {
              let s = colors[j];
              if (j !== 0)
                s += ` ${((sums[j] / totalWeight) * 100).toFixed(0)}%`;
              if (j !== colors.length - 1) {
                s += `, ${colors[j]} ${(
                  (sums[j + 1] / totalWeight) *
                  100
                ).toFixed(0)}%`;
              }
              segments.push(s);
            }

            style.backgroundImage = `linear-gradient(to right, ${segments.join(
              ', '
            )})`;
          }
          return { style };
        },
      },
    },
    {
      name: 'transactions',
      label: 'Konten',
      options: {
        customBodyRenderLite: i => {
          return (
            <div style={{ margin: theme.spacing(2) }}>
              {map(
                displayedResponse.records[i].meta.accountsByAmount,
                ({ accountId }) => {
                  return displayAccountById(
                    accounts,
                    users,
                    userInfo,
                    accountId
                  );
                }
              ).join(', ')}
            </div>
          );
        },
        sort: false,
        filterOptions: {
          names: map(
            userInfo
              ? filter(
                  accounts.items,
                  a => a.info.userId === userInfo.id && !a.info.hidden
                )
              : [],
            a => a.info.name
          ),
        },
        setCellProps: (_, i) => {
          const data = map(
            displayedResponse.records[i].meta.accountsByAmount,
            x => ({
              ...x,
              color:
                findAccount(accounts, x.accountId)?.info?.color || '#FFFFFF00',
            })
          );

          const style = { padding: 0 };
          if (data.length === 0) return { style };

          if (
            tinycolor(data[0].color).isValid() &&
            tinycolor(data[0].color).getBrightness() <= 120
          )
            style.color = '#F7F7F7';

          if (data.length === 1) {
            style.backgroundColor = data[0].color;
          } else {
            const colors = [];
            const weights = [];
            const sums = [0];

            data.forEach(d => {
              const w = Math.abs(d.amount);
              colors.push(d.color);
              weights.push(w);
              sums.push(sums[sums.length - 1] + w);
            });

            const totalWeight = weights.reduce((a, b) => a + b, 0);
            const segments = [];
            for (let j = 0; j < colors.length; j += 1) {
              let s = colors[j];
              if (j !== 0)
                s += ` ${((sums[j] / totalWeight) * 100).toFixed(0)}%`;
              if (j !== colors.length - 1) {
                s += `, ${colors[j]} ${(
                  (sums[j + 1] / totalWeight) *
                  100
                ).toFixed(0)}%`;
              }
              segments.push(s);
            }

            style.backgroundImage = `linear-gradient(to right, ${segments.join(
              ', '
            )})`;
          }
          return { style };
        },
      },
    },
    {
      name: 'totalAmount',
      label: 'Betrag',
      options: {
        customBodyRenderLite: i => {
          const amount = displayedResponse.records[i].totalAmount;

          let color = '';
          // eslint-disable-next-line prefer-destructuring
          if (amount < 0) color = deepOrange[800];
          // eslint-disable-next-line prefer-destructuring
          if (amount > 0) color = green[800];

          return (
            <Typography variant="inherit" style={{ color }}>
              {`${(amount / 100.0).toFixed(2)}€`}
            </Typography>
          );
        },
        filter: false,
        sort: false,
      },
    },
    {
      name: 'info.store',
      label: 'Geschäft',
      options: {
        display: mdUp,
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
        display: mdUp,
        filter: false,
      },
    },
    {
      name: 'info.comments',
      label: 'Kommentare',
      options: {
        display: mdUp,
        filter: false,
      },
    },
    {
      name: 'info.isPreliminary',
      label: 'Vorläufig',
      options: {
        display: false,
        filterType: 'dropdown',
        customBodyRender: v => (v ? 'Ja' : 'Nein'),
        customFilterListOptions: {
          render: v => (v === 'Ja' ? 'vorläufig' : 'nicht vorläufig'),
        },
        filterOptions: {
          names: ['Ja', 'Nein'],
        },
      },
    },
    {
      name: 'info.isTaxRelevant',
      label: 'Steuerrelevant',
      options: {
        display: false,
        filterType: 'dropdown',
        customBodyRender: v => (v ? 'Ja' : 'Nein'),
        customFilterListOptions: {
          render: v => (v === 'Ja' ? 'steuerrelevant' : 'nicht steuerrelevant'),
        },
        filterOptions: {
          names: ['Ja', 'Nein'],
        },
      },
    },
    {
      name: 'info.isTemplate',
      label: 'Vorlage',
      options: {
        filterType: 'dropdown',
        display: false,
        customBodyRender: v => (v ? 'Ja' : 'Nein'),
        customFilterListOptions: {
          render: v => (v === 'Ja' ? 'Vorlage' : 'keine Vorlage'),
        },
        filterOptions: {
          names: ['Ja', 'Nein'],
        },
      },
    },
    {
      name: 'info.isUnchecked',
      label: 'Unüberprüft',
      options: {
        filterType: 'dropdown',
        display: false,
        customBodyRender: v => (v ? 'Ja' : 'Nein'),
        customFilterListOptions: {
          render: v => (v === 'Ja' ? 'überprüft' : 'nicht überprüft'),
        },
        filterOptions: {
          names: ['Ja', 'Nein'],
        },
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
    selectableRows: 'single',
    selectableRowsOnClick: true,
    selectToolbarPlacement: 'none',
    enableNestedDataAccess: '.',
    serverSide: true,
    print: false,
    expandableRowsHeader: false,
    expandableRows: mdUp,
    expandableRowsOnClick: true,
    rowsExpanded: expandedRow !== null ? [expandedRow] : [],
    onRowExpansionChange: (_curExpanded, allExpanded) => {
      if (allExpanded.length === 0) setExpandedRow(null);
      else setExpandedRow(allExpanded[allExpanded.length - 1].index);
    },
    renderExpandableRow: (rowData, rowMeta) => {
      const colSpan = rowData.length + 1;
      const expense = displayedResponse.records[rowMeta.dataIndex];

      return (
        <TableRow>
          <TableCell colSpan={colSpan}>
            <ExpenseDetails
              expense={expense}
              accounts={accounts}
              categories={categories}
              users={users}
              userInfo={userInfo}
            />
          </TableCell>
        </TableRow>
      );
    },
    onTableChange: (_action, tableState) => {
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
          column,
          direction:
            tableState.sortOrder.direction === 'asc'
              ? 'ascending'
              : 'descending',
        });
      }

      forEach(tableState.filterList, (v, i) => {
        if (v.length > 0) {
          newQuery.filterBy[columns[i].name] = map(v, x => {
            if (columns[i].name === 'transactions')
              return findAccountByName(accounts, x).info.id.toString();

            if (columns[i].name === 'categories')
              return findCategoryByName(categories, x).info.id.toString();

            if (columns[i].name.startsWith('info.is'))
              return x === 'Ja' ? 'true' : 'false';

            return x;
          });
          newQuery.filterBy[columns[i].name].sort();
        }
      });

      if (JSON.stringify(newQuery) !== JSON.stringify(currentQuery)) {
        setExpandedRow(null);
        setSelectedRow(null);
        setQuery([currentQuery, newQuery]);
      }
    },
    download: false,
    rowsPerPageOptions: mdUp ? [10, 15, 20, 50] : [15],
    rowsPerPage: currentQuery.rowsPerPage,
    page: currentQuery.page,
    count: displayedResponse.filteredRecordCount,
    sortOrder: currentSortOrder,
    filterType: 'multiselect',
    responsive: 'standard',
    onRowSelectionChange: (_currentRowsSelected, allRowsSelected) => {
      if (allRowsSelected.length === 0) setSelectedRow(null);
      else setSelectedRow(allRowsSelected[0].index);

      if (xsDown) {
        if (allRowsSelected.length === 0) setExpenseId(null);
        else {
          const { id } = displayedResponse.records[
            allRowsSelected[0].index
          ].info;
          setExpenseId(id);
        }
      }
    },
    customToolbar: () => (
      <>
        <Tooltip title="Ausgabe bearbeiten">
          <span>
            <IconButton
              onClick={() => {
                const { id } = displayedResponse.records[selectedRow].info;
                setExpenseId(id);
              }}
              disabled={selectedRow === null}
            >
              <EditIcon />
            </IconButton>
          </span>
        </Tooltip>
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
  updateUsers: PropTypes.func.isRequired,
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
  users: PropTypes.shape({
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
  updateUsers: () => dispatch(updateUsersAction()),
});

const mapStateToProps = state => {
  const { expenses, accounts, categories, authentication, users } = state;
  const { userInfo } = authentication;

  return {
    expenses,
    accounts,
    categories,
    userInfo,
    users,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Expenses);
