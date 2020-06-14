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
  deleteCategory as deleteCategoryAction,
  createCategory as createCategoryAction,
  updateCategory as updateCategoryAction,
} from '../actions/categories';

const CategoryDialog = ({
  initialCategory,
  open,
  onClose,
  deleteCategory,
  createCategory,
  updateCategory,
}) => {
  const toIBAN = s => {
    const iban = s.trim().toUpperCase();
    if (iban === '' || /^[A-Z]{2}[0-9]{2}[0-9A-Z]{12,30}$/.test(iban))
      return iban;

    return null;
  };

  const toCategory = t => {
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

  const [category, setCategory] = useState(toEditable(initialCategory));
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [triedToSave, setTriedToSave] = useState(false);

  const reset = () => {
    setCategory(toEditable(initialCategory));
    setTriedToSave(false);
  };

  const onDelete = () => {
    setConfirmOpen(false);
    deleteCategory(category.id);
    onClose();
  };

  const onSave = () => {
    const t = toCategory(category);

    if (t === null) {
      setTriedToSave(true);
      return;
    }

    if (t.id === null) {
      createCategory(t);
    } else {
      updateCategory(t);
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
          {category.id !== null ? 'Depot bearbeiten' : 'Depot erstellen'}
        </DialogTitle>
        <DialogContent>
          <Grid container spacing={1}>
            <Grid item xs={12}>
              <TextField
                margin="dense"
                label="Name"
                fullWidth
                value={category.name}
                onChange={v =>
                  setCategory({
                    ...category,
                    name: v.target.value,
                  })
                }
                error={category.name.trim() === '' && triedToSave}
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                margin="dense"
                label="IBAN"
                fullWidth
                value={category.iban}
                onChange={v =>
                  setCategory({
                    ...category,
                    iban: v.target.value,
                  })
                }
                error={toIBAN(category.iban) === null && triedToSave}
              />
            </Grid>
          </Grid>
          {toCategory(category) === null && triedToSave && (
            <Alert severity="error">Depot ist unvollständig!</Alert>
          )}
        </DialogContent>
        <DialogActions>
          {category.id !== null && (
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

CategoryDialog.propTypes = {
  initialCategory: PropTypes.shape({
    id: PropTypes.number,
  }).isRequired,
  open: PropTypes.bool.isRequired,
  onClose: PropTypes.func.isRequired,
  deleteCategory: PropTypes.func.isRequired,
  updateCategory: PropTypes.func.isRequired,
  createCategory: PropTypes.func.isRequired,
};

const mapDispatchToProps = dispatch => ({
  deleteCategory: id => dispatch(deleteCategoryAction(id)),
  createCategory: t => dispatch(createCategoryAction(t)),
  updateCategory: t => dispatch(updateCategoryAction(t)),
});

export default connect(null, mapDispatchToProps)(CategoryDialog);
