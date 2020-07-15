import React from 'react';
import PropTypes from 'prop-types';
import {
  Paper,
  Grid,
  TableContainer,
  Table,
  TableCell,
  TableHead,
  TableRow,
  TableBody,
  useTheme,
  Typography,
  Divider,
  makeStyles,
} from '@material-ui/core';
import { some, zip } from 'lodash';
import { deepOrange, green } from '@material-ui/core/colors';
import tinycolor from 'tinycolor2';
import * as moment from 'moment';

import { findAccount, displayAccount } from '../selectors/accounts';
import { displayCategory, findCategory } from '../selectors/categories';
import { formatAmount } from '../utils';
import { formatBooking } from '../selectors/expenses';

const useStyles = makeStyles(() => ({
  heading: {
    fontWeight: 800,
  },
}));

const ExpenseDetails = ({ expense, accounts, categories, users, userInfo }) => {
  const theme = useTheme();
  const classes = useStyles();

  const transactionsWithComments = some(
    expense.transactions,
    t => t.comments && t.comments.length > 0
  );

  // TODO: (how to) display receipts
  // TODO: display events
  // TODO: display flags
  // TODO: use two columns for info display?

  return (
    <Grid container spacing={4} style={{ margin: theme.spacing(1) }}>
      {/* <Grid item xs={10}>
        Data: {JSON.stringify(expense)}
      </Grid> */}
      <Grid item xs={10}>
        <p>
          <Typography
            variant="inherit"
            className={classes.heading}
            paragraph={false}
          >
            ID
          </Typography>
          <br />
          {expense.info.id}
        </p>
        <Divider />
        <p>
          <Typography
            variant="inherit"
            className={classes.heading}
            paragraph={false}
          >
            Titel
          </Typography>
          <br />
          {expense.info.title}
        </p>
        <Divider />
        <p>
          <Typography
            variant="inherit"
            className={classes.heading}
            paragraph={false}
          >
            Effektiver Gesamtbetrag
          </Typography>
          <br />
          {formatAmount(expense.totalAmount)}
        </p>{' '}
        <Divider />
        <p>
          <Typography
            variant="inherit"
            className={classes.heading}
            paragraph={false}
          >
            {moment(expense.info.bookingStart).isSame(
              moment(expense.info.bookingEnd)
            )
              ? 'Buchungsdatum'
              : 'Buchungszeitraum'}
          </Typography>
          <br />
          {formatBooking(expense)}
        </p>
        {expense.info.store && (
          <>
            <Divider />
            <p>
              <Typography
                variant="inherit"
                className={classes.heading}
                paragraph={false}
              >
                Geschäft
              </Typography>
              <br />
              {expense.info.store}
            </p>
          </>
        )}
        {expense.info.description && (
          <>
            <Divider />
            <p>
              <Typography
                variant="inherit"
                className={classes.heading}
                paragraph={false}
              >
                Beschreibung
              </Typography>
              <br />
              {expense.info.description}
            </p>
          </>
        )}
        {expense.info.comments && (
          <>
            <Divider />
            <p>
              <Typography
                variant="inherit"
                className={classes.heading}
                paragraph={false}
              >
                Kommentare
              </Typography>
              <br />
              {expense.info.comments}
            </p>
          </>
        )}
      </Grid>
      <Grid item xs={10}>
        <TableContainer component={Paper}>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell style={{ width: '20rem' }}>Konto</TableCell>
                <TableCell style={{ width: '10rem' }} align="right">
                  Datum
                </TableCell>
                <TableCell style={{ width: '15rem' }} align="right">
                  Betrag
                </TableCell>
                <TableCell>Text</TableCell>
                {transactionsWithComments && <TableCell>Kommentare</TableCell>}
              </TableRow>
            </TableHead>
            <TableBody>
              {zip(expense.transactions, expense.calculatedAmounts).map(
                ([t, amount]) => {
                  const account = findAccount(accounts, t.accountId);

                  let amountColor = '';
                  // eslint-disable-next-line prefer-destructuring
                  if (amount < 0) amountColor = deepOrange[800];
                  // eslint-disable-next-line prefer-destructuring
                  if (amount > 0) amountColor = green[800];

                  let amountText = formatAmount(amount);
                  if (t.fraction !== null)
                    amountText = `${(t.fraction * 100).toFixed(
                      2
                    )}%  (${amountText})`;

                  let fgColor = '';
                  if (
                    tinycolor(account.info.color).isValid() &&
                    tinycolor(account.info.color).getBrightness() <= 120
                  )
                    fgColor = '#F7F7F7';

                  return (
                    <TableRow key={t.id}>
                      <TableCell
                        component="th"
                        scope="row"
                        style={{
                          backgroundColor: account.info.color,
                          color: fgColor,
                        }}
                      >
                        {displayAccount(users, userInfo, t.accountId, account)}
                      </TableCell>
                      <TableCell align="right">
                        {moment(t.date).format('L LT')}
                      </TableCell>
                      <TableCell align="right">
                        <span style={{ color: amountColor }}>{amountText}</span>
                      </TableCell>
                      <TableCell>
                        <span
                          style={{
                            fontFamily: theme.typography.fontFamilyMonospaced,
                            fontSize: '0.75rem',
                          }}
                        >
                          {t.statement}
                        </span>
                      </TableCell>
                      {transactionsWithComments && (
                        <TableCell>{t.comments}</TableCell>
                      )}
                    </TableRow>
                  );
                }
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Grid>
      {expense.categories.length > 0 && (
        <Grid item xs={3}>
          <TableContainer component={Paper}>
            <Table size="small">
              <TableHead>
                <TableRow>
                  <TableCell style={{ width: '20rem' }}>Kategorie</TableCell>
                  <TableCell align="right">Gewicht</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {expense.categories.map(c => {
                  const category = findCategory(categories, c.categoryId);

                  let fgColor = '';
                  if (
                    tinycolor(category.info.color).isValid() &&
                    tinycolor(category.info.color).getBrightness() <= 120
                  )
                    fgColor = '#F7F7F7';

                  return (
                    <TableRow key={c.id}>
                      <TableCell
                        component="th"
                        scope="row"
                        style={{
                          backgroundColor: category.info.color,
                          color: fgColor,
                        }}
                      >
                        {displayCategory(
                          users,
                          userInfo,
                          c.categoryId,
                          category
                        )}
                      </TableCell>
                      <TableCell align="right">{c.weight.toFixed(2)}</TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </TableContainer>
        </Grid>
      )}
    </Grid>
  );
};

ExpenseDetails.propTypes = {
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
  }).isRequired,
  expense: PropTypes.shape({
    info: PropTypes.shape({
      store: PropTypes.string,
      description: PropTypes.string,
      comments: PropTypes.string,
      id: PropTypes.number,
      title: PropTypes.string,
      bookingStart: PropTypes.string,
      bookingEnd: PropTypes.string,
    }),
    transactions: PropTypes.arrayOf(PropTypes.shape({})),
    categories: PropTypes.arrayOf(PropTypes.shape({})),
    calculatedAmounts: PropTypes.arrayOf(PropTypes.shape({})),
    totalAmount: PropTypes.number,
  }).isRequired,
};

export default ExpenseDetails;
