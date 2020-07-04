import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Grid,
  TextField,
  useMediaQuery,
  useTheme,
} from '@material-ui/core';
import Alert from '@material-ui/lab/Alert';
import PropTypes from 'prop-types';
import React, { useState } from 'react';
import { connect } from 'react-redux';
import DeleteIcon from '@material-ui/icons/Delete';

import {
  deleteExpense as deleteExpenseAction,
  createExpense as createExpenseAction,
  updateExpense as updateExpenseAction,
} from '../actions/expenses';

const ExpenseDialog = ({
  initialExpense,
  open,
  onClose,
  deleteExpense,
  createExpense,
  updateExpense,
}) => {
  const toIBAN = s => {
    const iban = s.trim().toUpperCase();
    if (iban === '' || /^[A-Z]{2}[0-9]{2}[0-9A-Z]{12,30}$/.test(iban))
      return iban;

    return null;
  };

  const toExpense = t => {
    const iban = toIBAN(t.iban);
    if (t.name.trim() === '' || iban === null) return null;

    const ret = {
      userId: -1,
      id: t.id,
      name: t.name.trim(),
      iban: iban !== '' ? iban : null,
    };

    return ret;
  };

  const toEditable = t => {
    return { ...t };
  };

  const theme = useTheme();
  const mdUp = useMediaQuery(theme.breakpoints.up('md'));

  const [expense, setExpense] = useState(toEditable(initialExpense));
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [triedToSave, setTriedToSave] = useState(false);

  const reset = () => {
    setExpense(toEditable(initialExpense));
    setTriedToSave(false);
  };

  const onDelete = () => {
    setConfirmOpen(false);
    deleteExpense(expense.id);
    onClose();
  };

  const onSave = () => {
    const t = toExpense(expense);

    if (t === null) {
      setTriedToSave(true);
      return;
    }

    if (t.id === null) {
      createExpense(t);
    } else {
      updateExpense(t);
    }

    onClose();
  };

  return (
    <>
      <Dialog
        fullScreen={!mdUp}
        open={open}
        disableBackdropClick
        onClose={() => {
          reset();
          onClose();
        }}
        onEntered={reset}
        onEnter={reset}
      >
        <DialogTitle id="form-dialog-title">
          {expense.id !== null ? 'Depot bearbeiten' : 'Depot erstellen'}
        </DialogTitle>
        <DialogContent>
          <Grid container spacing={1}>
            <Grid item xs={12}>
              <TextField
                margin="dense"
                label="Name"
                fullWidth
                value={expense.name}
                onChange={v =>
                  setExpense({
                    ...expense,
                    name: v.target.value,
                  })
                }
                error={expense.name.trim() === '' && triedToSave}
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                margin="dense"
                label="IBAN"
                fullWidth
                value={expense.iban}
                onChange={v =>
                  setExpense({
                    ...expense,
                    iban: v.target.value,
                  })
                }
                error={toIBAN(expense.iban) === null && triedToSave}
              />
            </Grid>
          </Grid>
          {toExpense(expense) === null && triedToSave && (
            <Alert severity="error">Depot ist unvollständig!</Alert>
          )}
        </DialogContent>
        <DialogActions>
          {expense.id !== null && (
            <Button
              variant="outlined"
              color="secondary"
              startIcon={<DeleteIcon />}
              onClick={() => setConfirmOpen(true)}
            >
              Löschen
            </Button>
          )}
          <Button color="primary" onClick={onClose}>
            Abbrechen
          </Button>
          <Button color="primary" onClick={onSave}>
            Speichern
          </Button>
        </DialogActions>
      </Dialog>
      <Dialog
        open={confirmOpen}
        onClose={() => setConfirmOpen(false)}
        aria-labelledby="alert-dialog-title"
        aria-describedby="alert-dialog-description"
      >
        <DialogContent>
          <DialogContentText id="alert-dialog-description">
            Depot wirklich löschen? Dies würde auch alle Transaktionen dieses
            Depots löschen!
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => setConfirmOpen(false)}
            color="primary"
            autoFocus
          >
            Nein
          </Button>
          <Button onClick={onDelete} color="primary">
            Ja
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};

ExpenseDialog.propTypes = {
  initialExpense: PropTypes.shape({
    id: PropTypes.number,
  }).isRequired,
  open: PropTypes.bool.isRequired,
  onClose: PropTypes.func.isRequired,
  deleteExpense: PropTypes.func.isRequired,
  updateExpense: PropTypes.func.isRequired,
  createExpense: PropTypes.func.isRequired,
};

const mapDispatchToProps = dispatch => ({
  deleteExpense: id => dispatch(deleteExpenseAction(id)),
  createExpense: t => dispatch(createExpenseAction(t)),
  updateExpense: t => dispatch(updateExpenseAction(t)),
});

export default connect(null, mapDispatchToProps)(ExpenseDialog);
