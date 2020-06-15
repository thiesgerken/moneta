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

import { updateCategories as updateCategoriesAction } from '../actions/categories';
import CategoryDialog from './CategoryDialog';

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

const Categories = ({ categories, updateCategories }) => {
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
      name: 'info.id',
      label: 'ID',
      options: {
        display: false,
      },
    },
    {
      name: 'info.name',
      label: 'Name',
      options: {
        sortOrder: 'asc',
      },
    },
    {
      name: 'info.description',
      label: 'Beschreibung',
    },
    {
      name: 'info.parent',
      label: 'Oberkategorie',
    },
    {
      name: 'info.color',
      label: 'Farbe',
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
    download: false,
    filter: false,
    rowsPerPageOptions: mdUp ? [10, 25, 50] : [10],
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
    textLabels: {
      body: {
        noMatch: 'Keine Kategorien vorhanden.',
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
                title="Kategorien"
                data={categories.items}
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
  const { categories } = state;

  return {
    categories,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Categories);
