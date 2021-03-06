import { connect } from 'react-redux';
import React, { useEffect } from 'react';
import PropTypes from 'prop-types';
import MUIDataTable from 'mui-datatables';
import { find, filter } from 'lodash';
import tinycolor from 'tinycolor2';
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

import { updateAccounts as updateAccountsAction } from '../actions/accounts';
import AccountDialog from './AccountDialog';
import { muiDatatablesLang } from '../lang';

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

const Accounts = ({ userId, accounts, updateAccounts }) => {
  const [accountId, setAccountId] = useQuery('id');
  const [page, setPage] = useQuery('page');

  let initialAccount = {
    id: null,
    userId: -1,
    name: '',
    iban: '',
  };
  if (
    accounts.items !== null &&
    accountId !== null &&
    parseInt(accountId, 10) >= 0
  ) {
    const a = find(accounts.items, x => x.id === parseInt(accountId, 10));
    if (a !== undefined) initialAccount = a;
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
    updateAccounts();
  });

  const isLoading = accounts.isFetching;

  const columns = [
    {
      name: 'info.color',
      options: {
        filter: false,
        sort: false,
        viewColumns: false,
        display: false,
      },
    },
    {
      name: 'info.id',
      label: 'ID',
      options: {
        display: false,
        filter: false,
      },
    },
    {
      name: 'info.name',
      label: 'Name',
      options: {
        filter: false,
        // customBodyRender: (v, { rowData }) => {
        //   const color = rowData[0];
        //   if (color === null || color === '') return v;

        //   return <ColorboxLabel color={color}>{v}</ColorboxLabel>;
        // },
      },
    },
    {
      name: 'info.color',
      label: 'Farbe',
      options: {
        sort: false,
        filter: false,
        setCellProps: v => {
          const c = tinycolor(v);
          if (!c.isValid()) return {};

          if (c.getBrightness() >= 120)
            return { style: { backgroundColor: v } };

          return { style: { backgroundColor: v, color: '#F7F7F7' } };
        },
      },
    },
    {
      name: 'info.description',
      label: 'Beschreibung',
      options: {
        sort: false,
        filter: false,
      },
    },
    {
      name: 'info.iban',
      label: 'IBAN',
      options: {
        filter: false,
      },
    },
    {
      name: 'meta.kind',
      label: 'Art',
    },
    {
      name: 'meta.availability',
      label: 'Verfügbarkeit',
    },
    {
      name: 'meta.risk',
      label: 'Risiko',
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
      if (rows.length > 0 && rows[0].dataIndex < accounts.items.length)
        setAccountId(accounts.items[rows[0].dataIndex].info.id);
      else setAccountId(null);
    },
    sortOrder: {
      name: 'info.name',
      direction: 'asc',
    },
    download: false,
    rowsPerPageOptions: mdUp ? [10, 15, 20, 50] : [15],
    responsive: 'standard',
    customToolbar: () => (
      <>
        <Tooltip title="Neues Konto">
          <IconButton onClick={() => setAccountId(-1)}>
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
                title="Konten"
                data={filter(accounts.items, c => c.info.userId === userId)}
                columns={columns}
                options={options}
              />
            </ThemeProvider>
          </div>
        </div>
      </Container>
      <AccountDialog
        initialAccount={initialAccount}
        open={accountId !== null && !isLoading}
        onClose={() => {
          setAccountId(null);
        }}
      />
    </div>
  );
};

Accounts.propTypes = {
  userId: PropTypes.number.isRequired,
  updateAccounts: PropTypes.func.isRequired,
  accounts: PropTypes.shape({
    items: PropTypes.arrayOf(PropTypes.object),
    isFetching: PropTypes.bool,
  }).isRequired,
};

const mapDispatchToProps = dispatch => ({
  updateAccounts: () => dispatch(updateAccountsAction()),
});

const mapStateToProps = state => {
  const { accounts, authentication } = state;

  return {
    accounts,
    userId: authentication.userInfo.id,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Accounts);
