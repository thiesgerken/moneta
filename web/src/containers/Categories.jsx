import { connect } from 'react-redux';
import React, { useEffect } from 'react';
import PropTypes from 'prop-types';
import MUIDataTable from 'mui-datatables';
import { map, find, filter } from 'lodash';
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

import { updateCategories as updateCategoriesAction } from '../actions/categories';
import CategoryDialog from './CategoryDialog';
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

const Categories = ({ userId, categories, updateCategories }) => {
  const [categoryId, setCategoryId] = useQuery('id');
  const [page, setPage] = useQuery('page');

  let initialCategory = {
    id: null,
    userId: -1,
    name: '',
    iban: '',
  };
  if (
    categories.items !== null &&
    categoryId !== null &&
    parseInt(categoryId, 10) >= 0
  ) {
    const a = find(categories.items, x => x.id === parseInt(categoryId, 10));
    if (a !== undefined) initialCategory = a;
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
    updateCategories();
  });

  const isLoading = categories.isFetching;

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
        filter: false,
        sort: false,
      },
    },
    {
      name: 'info.parent',
      label: 'Oberkategorie',
      options: {
        sort: false,
        customBodyRender: v => {
          if (v === null) return '';

          const c = find(categories.items, cc => cc.info.id === v);
          if (c === undefined) return v;

          // TODO: display as otheruser/name if the parent does not belong to this user
          return c.info.name;
        },
        display: mdUp,
      },
    },
    {
      name: 'replaces',
      label: 'Ersetzt',
      options: {
        filter: false,
        sort: false,
        customBodyRender: v => {
          return map(v, cId => {
            // TODO: display as otheruser/name if the parent does not belong to this user
            const c = find(categories.items, cc => cc.info.id === cId);
            if (c === undefined) return cId;
            return c.info.name;
          }).join(', ');
        },
        display: mdUp,
      },
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
      if (rows.length > 0 && rows[0].dataIndex < categories.items.length)
        setCategoryId(categories.items[rows[0].dataIndex].info.id);
      else setCategoryId(null);
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
        <Tooltip title="Neue Kategorie">
          <IconButton onClick={() => setCategoryId(-1)}>
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
                title="Kategorien"
                data={filter(categories.items, c => c.info.userId === userId)}
                columns={columns}
                options={options}
              />
            </ThemeProvider>
          </div>
        </div>
      </Container>
      <CategoryDialog
        initialCategory={initialCategory}
        open={categoryId !== null && !isLoading}
        onClose={() => {
          setCategoryId(null);
        }}
      />
    </div>
  );
};

Categories.propTypes = {
  userId: PropTypes.number.isRequired,
  updateCategories: PropTypes.func.isRequired,
  categories: PropTypes.shape({
    items: PropTypes.arrayOf(PropTypes.object),
    isFetching: PropTypes.bool,
  }).isRequired,
};

const mapDispatchToProps = dispatch => ({
  updateCategories: () => dispatch(updateCategoriesAction()),
});

const mapStateToProps = state => {
  const { categories, authentication } = state;

  return {
    categories,
    userId: authentication.userInfo.id,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Categories);
